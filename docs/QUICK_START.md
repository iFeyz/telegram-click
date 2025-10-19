# Quick Start Guide

## Prerequisites

### 1. Get Telegram Bot API Key
1. Open Telegram and search for **@BotFather**
2. Send `/newbot` command
3. Follow the instructions to choose a name and username
4. Copy the **API token** (looks like: `123456789:ABCdefGHIjklMNOpqrsTUVwxyz`)
5. Add the token to your `.env` file:
   ```bash
   TELOXIDE_TOKEN=your_bot_token_here
   ```

### 2. Start ngrok (For Public Access)
```bash
# Start ngrok to expose port 80
ngrok http 80

# Copy the HTTPS URL (e.g., https://abc123.ngrok.io)
# Update MINI_APP_URL in .env:
MINI_APP_URL=https://abc123.ngrok.io
```

## Running the Application

### 3. Build the Web App
```bash
cd mini-app
bun run build
cd ..
```

### 4. Build Rust Services for Linux
```bash
SQLX_OFFLINE=true ./scripts/build-linux.sh release
```

### 5. Start Database and Redis
```bash
docker-compose -f docker-compose.scaled.yml up -d postgres redis pgbouncer
```

### 6. Start All Services
```bash
docker-compose -f docker-compose.scaled.yml up -d
```

### 7. Test Your Bot
1. Open Telegram and find your bot
2. Send `/start` command
3. Click the "Play" button to open the mini-app
4. Start clicking!

## Verify Services are Running
```bash
docker-compose -f docker-compose.scaled.yml ps
```

## View Logs
```bash
# All services
docker-compose -f docker-compose.scaled.yml logs -f

# Specific service
docker-compose -f docker-compose.scaled.yml logs -f bot-service-1
```

## Stop All Services
```bash
docker-compose -f docker-compose.scaled.yml down
```
