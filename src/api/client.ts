import {
  CreateLocalRoomResponse,
  HostRoomDashboardResponse,
  LocalHelperHealth,
  LocalPartyRoom,
  PlayableCandidatePointer,
  UnifiedSongSubmissionResponse,
} from '../types';

const DEFAULT_LOCAL_HELPER_URL =
  (import.meta as any).env?.VITE_LOCAL_HELPER_URL || 'http://127.0.0.1:47821';

const STORAGE_KEYS = {
  HELPER_URL: 'dhun_local_helper_url',
  HOST_TOKEN: 'dhun_local_host_token',
  ROOM_ID: 'dhun_local_room_id',
  ROOM_QR_SVG: 'dhun_local_room_qr_svg',
  ROOM_GUEST_URL: 'dhun_local_room_guest_url',
};

/**
 * Client for communicating strictly with the host's Local DHUN Helper Service
 * running on the host laptop (`http://127.0.0.1:47821`).
 *
 * Notice:
 * - Zero cloud party management calls.
 * - Zero host approval/rejection endpoints.
 * - Zero direct access to the Global Shared Song Metadata Catalogue (engine-only).
 */
export class LocalHelperClient {
  private baseUrl: string;
  private hostToken: string | null;

  constructor() {
    this.baseUrl =
      (typeof window !== 'undefined' && localStorage.getItem(STORAGE_KEYS.HELPER_URL)) ||
      DEFAULT_LOCAL_HELPER_URL;
    this.hostToken =
      (typeof window !== 'undefined' && localStorage.getItem(STORAGE_KEYS.HOST_TOKEN)) || null;
  }

  public getBaseUrl(): string {
    return this.baseUrl;
  }

  public setBaseUrl(url: string): void {
    const clean = url.trim().replace(/\/+$/, '');
    this.baseUrl = clean || DEFAULT_LOCAL_HELPER_URL;
    if (typeof window !== 'undefined') {
      localStorage.setItem(STORAGE_KEYS.HELPER_URL, this.baseUrl);
    }
  }

  public getHostToken(): string | null {
    return this.hostToken;
  }

  public setHostToken(token: string | null): void {
    this.hostToken = token;
    if (typeof window !== 'undefined') {
      if (token) {
        localStorage.setItem(STORAGE_KEYS.HOST_TOKEN, token);
      } else {
        localStorage.removeItem(STORAGE_KEYS.HOST_TOKEN);
      }
    }
  }

  public getSavedRoomId(): string | null {
    if (typeof window === 'undefined') return null;
    return localStorage.getItem(STORAGE_KEYS.ROOM_ID);
  }

  public setSavedRoomSession(
    roomId: string | null,
    guestUrl?: string | null,
    qrSvg?: string | null
  ): void {
    if (typeof window === 'undefined') return;
    if (roomId) {
      localStorage.setItem(STORAGE_KEYS.ROOM_ID, roomId);
      if (guestUrl) localStorage.setItem(STORAGE_KEYS.ROOM_GUEST_URL, guestUrl);
      else localStorage.removeItem(STORAGE_KEYS.ROOM_GUEST_URL);
      if (qrSvg) localStorage.setItem(STORAGE_KEYS.ROOM_QR_SVG, qrSvg);
      else localStorage.removeItem(STORAGE_KEYS.ROOM_QR_SVG);
    } else {
      localStorage.removeItem(STORAGE_KEYS.ROOM_ID);
      localStorage.removeItem(STORAGE_KEYS.ROOM_GUEST_URL);
      localStorage.removeItem(STORAGE_KEYS.ROOM_QR_SVG);
    }
  }

  public getSavedGuestQr(): { guestUrl: string | null; qrSvg: string | null } {
    if (typeof window === 'undefined') return { guestUrl: null, qrSvg: null };
    return {
      guestUrl: localStorage.getItem(STORAGE_KEYS.ROOM_GUEST_URL),
      qrSvg: localStorage.getItem(STORAGE_KEYS.ROOM_QR_SVG),
    };
  }

  private async request<T>(
    path: string,
    options: RequestInit = {},
    includeHostAuth: boolean = false
  ): Promise<T> {
    const url = `${this.baseUrl}${path}`;
    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
      ...(options.headers as Record<string, string>),
    };

    if (includeHostAuth && this.hostToken) {
      headers['Authorization'] = `Bearer ${this.hostToken}`;
      headers['X-Host-Token'] = this.hostToken;
    }

    const res = await fetch(url, {
      ...options,
      headers,
    });

    if (!res.ok) {
      const errorText = await res.text().catch(() => res.statusText);
      throw new Error(errorText || `Local helper request failed (${res.status})`);
    }

    return res.json();
  }

  public async checkHealth(): Promise<{
    connected: boolean;
    health?: LocalHelperHealth;
    error?: string;
  }> {
    try {
      const controller = new AbortController();
      const timeoutId = setTimeout(() => controller.abort(), 3000);
      const res = await fetch(`${this.baseUrl}/api/local/health`, {
        method: 'GET',
        signal: controller.signal,
      });
      clearTimeout(timeoutId);

      if (!res.ok) {
        return { connected: false, error: `HTTP ${res.status}` };
      }
      const data: LocalHelperHealth = await res.json();
      return { connected: true, health: data };
    } catch (err: any) {
      return {
        connected: false,
        error:
          err?.name === 'AbortError'
            ? 'Local DHUN helper timed out'
            : 'Local DHUN helper is not running on this computer',
      };
    }
  }

  public async createLocalRoom(payload: {
    party_name: string;
    dj_level: number;
    primary_mood_id: string;
    secondary_mood_id?: string;
  }): Promise<CreateLocalRoomResponse> {
    const data = await this.request<CreateLocalRoomResponse>('/api/local/rooms', {
      method: 'POST',
      body: JSON.stringify(payload),
    });
    this.setHostToken(data.host_token);
    this.setSavedRoomSession(data.room.room_id, data.guest_url, data.qr_svg);
    return data;
  }

  public async getRoomDashboard(roomId: string): Promise<HostRoomDashboardResponse> {
    return this.request<HostRoomDashboardResponse>(
      `/api/local/rooms/${roomId}`,
      { method: 'GET' },
      true
    );
  }

  public async startLocalRoom(roomId: string): Promise<LocalPartyRoom> {
    return this.request<LocalPartyRoom>(
      `/api/local/rooms/${roomId}/start`,
      { method: 'POST' },
      true
    );
  }

  public async closeLocalRoom(
    roomId: string,
    forceReleaseAudio: boolean = false
  ): Promise<{
    room_id: string;
    status: string;
    closed_at: number;
    invalidated_guests: number;
    purged_cache_files: number;
    deferred_locked_files: number;
  }> {
    return this.request(
      `/api/local/rooms/${roomId}/close`,
      {
        method: 'POST',
        body: JSON.stringify({ force_release_audio: forceReleaseAudio }),
      },
      true
    );
  }

  public async releaseRoomCache(roomId: string): Promise<{
    room_id: string;
    purged_cache_files: number;
  }> {
    return this.request(
      `/api/local/rooms/${roomId}/cache/release`,
      { method: 'POST' },
      true
    );
  }

  /**
   * Fetches locally cached authorized audio bytes from the host-only streaming route:
   * `GET /api/local/rooms/:room_id/cache/:cache_id/stream`
   * Returns raw ArrayBuffer directly to WebAudioEngine (never persisted in React state).
   */
  public async fetchAuthorizedCachedAudio(
    roomId: string,
    cacheId: string
  ): Promise<ArrayBuffer> {
    if (!this.hostToken) {
      throw new Error('Host authentication token missing for protected cache stream.');
    }
    const url = `${this.baseUrl}/api/local/rooms/${roomId}/cache/${encodeURIComponent(cacheId)}/stream`;
    const res = await fetch(url, {
      method: 'GET',
      headers: {
        Authorization: `Bearer ${this.hostToken}`,
        'X-Host-Token': this.hostToken,
      },
    });
    if (!res.ok) {
      const msg = await res.text().catch(() => res.statusText);
      throw new Error(`Cache stream failed (${res.status}): ${msg}`);
    }
    return res.arrayBuffer();
  }

  public async recordPlaybackEvent(
    roomId: string,
    payload: {
      submission_id: string;
      event: 'prepared' | 'playing' | 'played' | 'decode_failed';
      deck_id?: number;
      error_detail?: string;
    }
  ): Promise<{
    submission_id: string;
    dj_status: string;
    cache_locked: boolean;
    next_playable_candidate: PlayableCandidatePointer | null;
    transition_style: string;
  }> {
    return this.request(
      `/api/local/rooms/${roomId}/dj/playback-event`,
      {
        method: 'POST',
        body: JSON.stringify(payload),
      },
      true
    );
  }

  public async updateRoomConfig(
    roomId: string,
    payload: {
      dj_level?: number;
      primary_mood_id?: string;
      secondary_mood_id?: string;
    }
  ): Promise<LocalPartyRoom> {
    return this.request<LocalPartyRoom>(
      `/api/local/rooms/${roomId}/config`,
      {
        method: 'POST',
        body: JSON.stringify(payload),
      },
      true
    );
  }

  /**
   * Unified song submission endpoint used by BOTH the host and LAN guests.
   * No approval queue — enters the private DJ candidate pool directly.
   */
  public async submitSong(roomId: string, url: string): Promise<UnifiedSongSubmissionResponse> {
    return this.request<UnifiedSongSubmissionResponse>(
      `/api/local/rooms/${roomId}/submissions`,
      {
        method: 'POST',
        body: JSON.stringify({ url }),
      },
      true
    );
  }

  public async evaluateDjPool(roomId: string): Promise<{
    evaluated_count: number;
    prepared_next_submission_id: string | null;
    next_playable_candidate: PlayableCandidatePointer | null;
    interrupted_current_track: boolean;
    transition_style: string;
  }> {
    return this.request(
      `/api/local/rooms/${roomId}/dj/evaluate`,
      { method: 'POST' },
      true
    );
  }

  public getHostWebSocketUrl(roomId: string): string | null {
    if (!this.hostToken) return null;
    const base = this.baseUrl.replace(/^http/, 'ws');
    return `${base}/ws/local/rooms/${roomId}?role=host&token=${encodeURIComponent(this.hostToken)}`;
  }
}

export const localHelperClient = new LocalHelperClient();
