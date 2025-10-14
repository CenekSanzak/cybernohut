import { useState, useEffect } from "react";

// Notification manager for global notifications
export interface NotificationItem {
  id: string;
  message: string;
  type: "success" | "error" | "info";
  duration?: number;
}

let notificationId = 0;
const notifications: NotificationItem[] = [];
const listeners: Array<(notifications: NotificationItem[]) => void> = [];

export const showNotification = (
  message: string,
  type: "success" | "error" | "info" = "info",
  duration = 3000
) => {
  const id = `notification-${++notificationId}`;
  const notification: NotificationItem = { id, message, type, duration };

  notifications.push(notification);
  listeners.forEach((listener) => listener([...notifications]));

  if (duration > 0) {
    setTimeout(() => {
      removeNotification(id);
    }, duration);
  }
};

export const removeNotification = (id: string) => {
  const index = notifications.findIndex((n) => n.id === id);
  if (index !== -1) {
    notifications.splice(index, 1);
    listeners.forEach((listener) => listener([...notifications]));
  }
};

export const useNotifications = () => {
  const [notificationList, setNotificationList] = useState<NotificationItem[]>(
    []
  );

  useEffect(() => {
    const listener = (notifications: NotificationItem[]) => {
      setNotificationList(notifications);
    };

    listeners.push(listener);
    setNotificationList([...notifications]);

    return () => {
      const index = listeners.indexOf(listener);
      if (index !== -1) {
        listeners.splice(index, 1);
      }
    };
  }, []);

  return notificationList;
};
