import React, { useState, useEffect } from "react";
import { X } from "lucide-react";
import { useNotifications, removeNotification } from "@/utils/notifications";

export interface NotificationProps {
  message: string;
  type?: "success" | "error" | "info";
  duration?: number;
  onClose?: () => void;
}

export const Notification: React.FC<NotificationProps> = ({
  message,
  type = "info",
  duration = 3000,
  onClose,
}) => {
  const [isVisible, setIsVisible] = useState(true);

  useEffect(() => {
    if (duration > 0) {
      const timer = setTimeout(() => {
        setIsVisible(false);
        setTimeout(() => onClose?.(), 300); // Wait for animation
      }, duration);

      return () => clearTimeout(timer);
    }
  }, [duration, onClose]);

  const handleClose = () => {
    setIsVisible(false);
    setTimeout(() => onClose?.(), 300);
  };

  const getTypeStyles = () => {
    switch (type) {
      case "success":
        return "bg-green-500 text-white border-green-600";
      case "error":
        return "bg-red-500 text-white border-red-600";
      default:
        return "bg-blue-500 text-white border-blue-600";
    }
  };

  if (!isVisible) return null;

  return (
    <div
      className={`fixed top-4 right-4 z-50 px-4 py-3 rounded-lg shadow-lg border transition-all duration-300 transform ${
        isVisible ? "translate-x-0 opacity-100" : "translate-x-full opacity-0"
      } ${getTypeStyles()}`}
    >
      <div className="flex items-center gap-3">
        <span className="text-sm font-medium">{message}</span>
        <button
          onClick={handleClose}
          className="ml-2 hover:bg-white/20 rounded p-1 transition-colors"
          aria-label="Close notification"
        >
          <X size={16} />
        </button>
      </div>
    </div>
  );
};

export const NotificationContainer: React.FC = () => {
  const notifications = useNotifications();

  return (
    <>
      {notifications.map((notification) => (
        <Notification
          key={notification.id}
          message={notification.message}
          type={notification.type}
          duration={notification.duration}
          onClose={() => removeNotification(notification.id)}
        />
      ))}
    </>
  );
};
