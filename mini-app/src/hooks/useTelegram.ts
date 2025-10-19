
import { useEffect, useState } from 'react';

export interface TelegramUser {
  id: number;
  firstName: string;
  lastName?: string;
  username?: string;
  photoUrl?: string;
}

const getTelegramWebApp = () => {
  if (typeof window !== 'undefined' && window.Telegram?.WebApp) {
    return window.Telegram.WebApp;
  }
  return null;
};

export function useTelegram() {
  const [user, setUser] = useState<TelegramUser | null>(null);
  const [isReady, setIsReady] = useState(false);

  useEffect(() => {
    const WebApp = getTelegramWebApp();

    if (!WebApp) {
      console.warn('Telegram WebApp not available');
      setIsReady(true);
      return;
    }

    WebApp.ready();
    WebApp.expand();

    const tgUser = WebApp.initDataUnsafe?.user;
    if (tgUser) {
      setUser({
        id: tgUser.id,
        firstName: tgUser.first_name,
        lastName: tgUser.last_name,
        username: tgUser.username,
        photoUrl: tgUser.photo_url,
      });
    }

    setIsReady(true);
  }, []);

  const hapticFeedback = (style: 'light' | 'medium' | 'heavy' = 'light') => {
    const WebApp = getTelegramWebApp();
    if (WebApp?.HapticFeedback) {
      WebApp.HapticFeedback.impactOccurred(style);
    }
  };

  const showAlert = (message: string) => {
    const WebApp = getTelegramWebApp();
    if (WebApp) {
      WebApp.showAlert(message);
    }
  };

  const WebApp = getTelegramWebApp();

  return {
    user,
    isReady,
    hapticFeedback,
    showAlert,
    themeParams: WebApp?.themeParams || {},
  };
}
