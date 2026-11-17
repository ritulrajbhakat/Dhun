import { PartyEventMessage } from '../types';
import { apiClient } from './client';

type EventHandler = (event: PartyEventMessage) => void;
type StatusHandler = (connected: boolean) => void;

export class PartyWebSocketClient {
  private ws: WebSocket | null = null;
  private partyId: string | null = null;
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private reconnectAttempts = 0;
  private shouldReconnect = false;
  private onEventCallback: EventHandler | null = null;
  private onStatusCallback: StatusHandler | null = null;

  public connect(partyId: string, onEvent: EventHandler, onStatus?: StatusHandler): void {
    this.disconnect();
    this.partyId = partyId;
    this.onEventCallback = onEvent;
    this.onStatusCallback = onStatus || null;
    this.shouldReconnect = true;
    this.reconnectAttempts = 0;
    this.openSocket();
  }

  private openSocket(): void {
    if (!this.partyId || !this.shouldReconnect) return;

    const url = apiClient.getWebSocketUrl(this.partyId);
    try {
      this.ws = new WebSocket(url);

      this.ws.onopen = () => {
        this.reconnectAttempts = 0;
        this.onStatusCallback?.(true);
      };

      this.ws.onmessage = (event) => {
        try {
          const parsed: PartyEventMessage = JSON.parse(event.data);
          this.onEventCallback?.(parsed);
        } catch (err) {
          console.warn('Failed to parse WebSocket message:', err);
        }
      };

      this.ws.onclose = () => {
        this.onStatusCallback?.(false);
        this.scheduleReconnect();
      };

      this.ws.onerror = () => {
        this.ws?.close();
      };
    } catch (err) {
      this.onStatusCallback?.(false);
      this.scheduleReconnect();
    }
  }

  private scheduleReconnect(): void {
    if (!this.shouldReconnect) return;
    if (this.reconnectTimer) clearTimeout(this.reconnectTimer);
    const delay = Math.min(1000 * Math.pow(2, this.reconnectAttempts), 15000);
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
    this.partyId = null;
    this.onStatusCallback?.(false);
  }
}

export const partyWsClient = new PartyWebSocketClient();
