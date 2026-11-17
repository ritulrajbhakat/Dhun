import { LocalRoomEvent } from '../types';
import { localHelperClient } from './client';

type EventHandler = (event: LocalRoomEvent) => void;
type StatusHandler = (connected: boolean) => void;

export class LocalRoomWebSocketClient {
  private ws: WebSocket | null = null;
  private roomId: string | null = null;
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private reconnectAttempts = 0;
  private shouldReconnect = false;
  private onEventCallback: EventHandler | null = null;
  private onStatusCallback: StatusHandler | null = null;

  public connect(roomId: string, onEvent: EventHandler, onStatus?: StatusHandler): void {
    this.disconnect();
    this.roomId = roomId;
    this.onEventCallback = onEvent;
    this.onStatusCallback = onStatus || null;
    this.shouldReconnect = true;
    this.reconnectAttempts = 0;
    this.openSocket();
  }

  private openSocket(): void {
    if (!this.roomId || !this.shouldReconnect) return;
    const url = localHelperClient.getHostWebSocketUrl(this.roomId);
    if (!url) return;

    try {
      this.ws = new WebSocket(url);

      this.ws.onopen = () => {
        this.reconnectAttempts = 0;
        this.onStatusCallback?.(true);
      };

      this.ws.onmessage = (event) => {
        try {
          const parsed: LocalRoomEvent = JSON.parse(event.data);
          this.onEventCallback?.(parsed);
        } catch (err) {
          console.warn('Failed to parse local room WebSocket message:', err);
        }
      };

      this.ws.onclose = () => {
        this.onStatusCallback?.(false);
        this.scheduleReconnect();
      };

      this.ws.onerror = () => {
        this.ws?.close();
      };
    } catch {
      this.onStatusCallback?.(false);
      this.scheduleReconnect();
    }
  }

  private scheduleReconnect(): void {
    if (!this.shouldReconnect) return;
    if (this.reconnectTimer) clearTimeout(this.reconnectTimer);
    const delay = Math.min(1000 * Math.pow(2, this.reconnectAttempts), 12000);
    this.reconnectAttempts++;
    this.reconnectTimer = setTimeout(() => this.openSocket(), delay);
  }

  public disconnect(): void {
    this.shouldReconnect = false;
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
    if (this.ws) {
      this.ws.onclose = null;
      this.ws.onerror = null;
      this.ws.onmessage = null;
      this.ws.close();
      this.ws = null;
    }
    this.roomId = null;
    this.onStatusCallback?.(false);
  }
}

export const localRoomWsClient = new LocalRoomWebSocketClient();
