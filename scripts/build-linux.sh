#!/bin/bash


set -e  


RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' 

print_info() {
    echo -e "${BLUE}ℹ  $1${NC}"
}

print_success() {
    echo -e "${GREEN} $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}  $1${NC}"
}

print_error() {
    echo -e "${RED} $1${NC}"
}

print_header() {
    echo ""
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BLUE}  $1${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""
}

print_header "Rust Cross-Compilation for Linux"


print_header "Step 1/6: Checking Prerequisites"

if ! command -v cargo &> /dev/null; then
    print_error "Rust is not installed. Please install Rust first."
    echo "Visit: https://rustup.rs/"
    exit 1
fi
print_success "Rust found: $(rustc --version)"

print_info "Checking for Linux musl target..."
if ! rustup target list --installed | grep -q "x86_64-unknown-linux-musl"; then
    print_warning "Linux musl target not installed. Installing..."
    rustup target add x86_64-unknown-linux-musl
    print_success "Installed x86_64-unknown-linux-musl target"
else
    print_success "Linux musl target already installed"
fi

if ! command -v x86_64-linux-musl-gcc &> /dev/null; then
    print_warning "musl-cross toolchain not found."
    print_info "Installing via Homebrew..."

    if ! command -v brew &> /dev/null; then
        print_error "Homebrew not installed. Please install Homebrew first."
        echo "Visit: https://brew.sh/"
        exit 1
    fi

    brew install filosottile/musl-cross/musl-cross
    print_success "Installed musl-cross toolchain"
else
    print_success "musl-cross toolchain found"
fi


print_header "Step 2/6: Setting Up Build Environment"

export CC_x86_64_unknown_linux_musl=x86_64-linux-musl-gcc
export CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER=x86_64-linux-musl-gcc

export DATABASE_URL=${DATABASE_URL:-"postgres://postgres:password@localhost:5432/clickgame"}

print_success "Environment configured for cross-compilation"


print_header "Step 3/6: Regenerating Proto Files"

print_info "Cleaning and rebuilding shared package to regenerate proto definitions..."
cargo clean -p shared

print_info "Building shared package (this will regenerate proto files)..."
cargo build -p shared $([[ "${1:-release}" == "release" ]] && echo "--release" || echo "")

if [[ $? -eq 0 ]]; then
    print_success "Proto files regenerated successfully"
else
    print_error "Failed to regenerate proto files"
    exit 1
fi


print_header "Step 4/6: Building Services"

BUILD_MODE=${1:-release}

if [[ "$BUILD_MODE" == "release" ]]; then
    print_info "Building in RELEASE mode (optimized, slower build)..."
    CARGO_FLAGS="--release"
    TARGET_DIR="target/x86_64-unknown-linux-musl/release"
else
    print_info "Building in DEBUG mode (faster build, larger binaries)..."
    CARGO_FLAGS=""
    TARGET_DIR="target/x86_64-unknown-linux-musl/debug"
fi

SERVICES=("bot-service" "game-service" "leaderboard-service")


export OPENSSL_STATIC=1
export OPENSSL_VENDOR=1

for service in "${SERVICES[@]}"; do
    print_info "Building $service..."
    cargo build \
        --target x86_64-unknown-linux-musl \
        --bin "$service" \
        --features vendored \
        $CARGO_FLAGS

    if [[ $? -eq 0 ]]; then
        print_success "$service built successfully"
    else
        print_error "Failed to build $service"
        exit 1
    fi
done


print_header "Step 5/6: Organizing Binaries"

DEPLOY_DIR="target/linux-deploy"
mkdir -p "$DEPLOY_DIR"

for service in "${SERVICES[@]}"; do
    cp "$TARGET_DIR/$service" "$DEPLOY_DIR/"
    print_success "Copied $service to $DEPLOY_DIR/"
done

print_header "Step 6/6: Verifying Binaries"

for service in "${SERVICES[@]}"; do
    BINARY="$DEPLOY_DIR/$service"

    if [[ ! -f "$BINARY" ]]; then
        print_error "$service binary not found"
        exit 1
    fi

    SIZE=$(ls -lh "$BINARY" | awk '{print $5}')

    if file "$BINARY" | grep -q "ELF 64-bit"; then
        print_success "$service: $SIZE (Linux ELF binary)"
    else
        print_error "$service: Not a valid Linux binary"
        exit 1
    fi
done


print_header "Build Complete "

echo ""
print_success "All services built successfully for Linux (x86_64-musl)"
echo ""
echo -e "${GREEN}Built binaries:${NC}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
for service in "${SERVICES[@]}"; do
    echo "   $DEPLOY_DIR/$service"
done
echo ""



if [[ "$BUILD_MODE" == "release" ]]; then
    echo -e "${GREEN}Build mode: RELEASE${NC} (optimized for production)"
else
    echo -e "${YELLOW}Build mode: DEBUG${NC} (faster builds, larger binaries)"
    echo "   For production, run: ./scripts/build-linux.sh release"
fi

echo ""
print_success " Cross-compilation completed successfully!"
echo ""
