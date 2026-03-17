import { useEffect, useState, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import { DOMAIN_EVENT, type DomainEvent, type NotificationPayload } from "@cmux-win/protocol";
import { getNotifications, getUnreadCount, markNotificationRead } from "../lib/desktopClient";

export function NotificationCenter() {
  const [notifications, setNotifications] = useState<NotificationPayload[]>([]);
  const [unreadCount, setUnreadCount] = useState(0);
  const [isOpen, setIsOpen] = useState(false);

  const refresh = useCallback(() => {
    getNotifications()
      .then(setNotifications)
      .catch(() => {});
    getUnreadCount()
      .then(setUnreadCount)
      .catch(() => {});
  }, []);

  useEffect(() => {
    refresh();
    const unlistenPromise = listen<DomainEvent>(DOMAIN_EVENT, (event) => {
      if (event.payload.type === "notificationCreated") {
        refresh();
      }
    });
    return () => {
      unlistenPromise.then((unlisten) => unlisten());
    };
  }, [refresh]);

  const handleMarkRead = (id: string) => {
    markNotificationRead(id)
      .then(() => refresh())
      .catch(() => {});
  };

  return (
    <div className="notification-center">
      <button
        type="button"
        className="notification-badge"
        aria-label="Notifications"
        onClick={() => setIsOpen((prev) => !prev)}
      >
        Notifications{unreadCount > 0 ? ` (${unreadCount})` : ""}
      </button>
      {isOpen ? (
        <div className="notification-panel" role="list" aria-label="Notification list">
          {notifications.length === 0 ? (
            <p className="notification-empty">No notifications</p>
          ) : (
            notifications.map((n) => (
              <div
                key={n.id}
                className={`notification-item${n.read ? "" : " notification-unread"}`}
                role="listitem"
              >
                <div className="notification-item-header">
                  <strong>{n.title}</strong>
                  <span className={`notification-level notification-level-${n.level}`}>
                    {n.level}
                  </span>
                </div>
                <p>{n.body}</p>
                {!n.read ? (
                  <button
                    type="button"
                    className="secondary-button"
                    onClick={() => handleMarkRead(n.id)}
                  >
                    Mark read
                  </button>
                ) : null}
              </div>
            ))
          )}
        </div>
      ) : null}
    </div>
  );
}
