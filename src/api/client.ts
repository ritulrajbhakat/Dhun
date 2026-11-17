import {
  CloudParty,
  CreatePartyResponse,
  PartyConfiguration,
  PartyDetailsResponse,
  PartySongRequest,
  PartySourceReference,
} from '../types';

const DEFAULT_API_BASE = (import.meta as any).env?.VITE_API_BASE_URL || 'http://localhost:8080';

const STORAGE_KEYS = {
  API_BASE_URL: 'dhun_api_base_url',
  HOST_TOKEN: 'dhun_host_token',
  PARTY_ID: 'dhun_active_party_id',
};

export class DhunApiClient {
  private baseUrl: string;
  private hostToken: string | null;

  constructor() {
    this.baseUrl =
      (typeof window !== 'undefined' && localStorage.getItem(STORAGE_KEYS.API_BASE_URL)) ||
      DEFAULT_API_BASE;
    this.hostToken =
      (typeof window !== 'undefined' && localStorage.getItem(STORAGE_KEYS.HOST_TOKEN)) || null;
  }

  public getBaseUrl(): string {
    return this.baseUrl;
  }

  public setBaseUrl(url: string): void {
    const clean = url.trim().replace(/\/+$/, '');
    this.baseUrl = clean || DEFAULT_API_BASE;
    if (typeof window !== 'undefined') {
      localStorage.setItem(STORAGE_KEYS.API_BASE_URL, this.baseUrl);
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

  public getSavedPartyId(): string | null {
    if (typeof window === 'undefined') return null;
    return localStorage.getItem(STORAGE_KEYS.PARTY_ID);
  }

  public setSavedPartyId(partyId: string | null): void {
    if (typeof window === 'undefined') return;
    if (partyId) {
      localStorage.setItem(STORAGE_KEYS.PARTY_ID, partyId);
    } else {
      localStorage.removeItem(STORAGE_KEYS.PARTY_ID);
    }
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
      throw new Error(errorText || `Request failed with status ${res.status}`);
    }

    return res.json();
  }

  public async checkHealth(): Promise<{ connected: boolean; version?: string; error?: string }> {
    try {
      const controller = new AbortController();
      const timeoutId = setTimeout(() => controller.abort(), 3500);
      const res = await fetch(`${this.baseUrl}/api/health`, {
        method: 'GET',
        signal: controller.signal,
      });
      clearTimeout(timeoutId);

      if (!res.ok) {
        return { connected: false, error: `HTTP ${res.status}` };
      }
      const data = await res.json();
      return { connected: true, version: data.version || '1.0.0' };
    } catch (err: any) {
      return {
        connected: false,
        error: err?.name === 'AbortError' ? 'Connection timed out' : (err?.message || 'Backend unreachable'),
      };
    }
  }

  public async createParty(payload: {
    party_name: string;
    dj_level?: string;
    primary_mood_id?: string;
    secondary_mood_id?: string;
    energy_progression?: string;
    allow_guest_requests?: boolean;
    max_requests_per_guest?: number;
  }): Promise<CreatePartyResponse> {
    const data = await this.request<CreatePartyResponse>('/api/parties', {
      method: 'POST',
      body: JSON.stringify(payload),
    });
    this.setHostToken(data.host_token);
    this.setSavedPartyId(data.party_id);
    return data;
  }

  public async getParty(partyId: string): Promise<PartyDetailsResponse> {
    return this.request<PartyDetailsResponse>(`/api/parties/${partyId}`, {
      method: 'GET',
    });
  }

  public async startParty(partyId: string): Promise<CloudParty> {
    return this.request<CloudParty>(
      `/api/parties/${partyId}/start`,
      { method: 'POST' },
      true
    );
  }

  public async endParty(partyId: string): Promise<CloudParty> {
    return this.request<CloudParty>(
      `/api/parties/${partyId}/end`,
      { method: 'POST' },
      true
    );
  }

  public async sendHeartbeat(partyId: string): Promise<{ status: string; last_heartbeat_at: number }> {
    return this.request<{ status: string; last_heartbeat_at: number }>(
      `/api/parties/${partyId}/heartbeat`,
      { method: 'POST' },
      true
    );
  }

  public async updateConfig(
    partyId: string,
    payload: Partial<PartyConfiguration>
  ): Promise<PartyConfiguration> {
    return this.request<PartyConfiguration>(
      `/api/parties/${partyId}/config`,
      {
        method: 'POST',
        body: JSON.stringify(payload),
      },
      true
    );
  }

  public async registerSource(partyId: string, url: string): Promise<PartySourceReference> {
    return this.request<PartySourceReference>(
      `/api/parties/${partyId}/sources`,
      {
        method: 'POST',
        body: JSON.stringify({ url }),
      },
      true
    );
  }

  public async listSources(partyId: string): Promise<PartySourceReference[]> {
    return this.request<PartySourceReference[]>(`/api/parties/${partyId}/sources`, {
      method: 'GET',
    });
  }

  public async listRequests(partyId: string): Promise<PartySongRequest[]> {
    return this.request<PartySongRequest[]>(
      `/api/parties/${partyId}/requests`,
      { method: 'GET' },
      true
    );
  }

  public async approveRequest(partyId: string, requestId: string): Promise<void> {
    await this.request(
      `/api/parties/${partyId}/requests/${requestId}/approve`,
      { method: 'POST' },
      true
    );
  }

  public async rejectRequest(partyId: string, requestId: string, reason?: string): Promise<void> {
    await this.request(
      `/api/parties/${partyId}/requests/${requestId}/reject`,
      {
        method: 'POST',
        body: JSON.stringify({ reason }),
      },
      true
    );
  }

  public getWebSocketUrl(partyId: string): string {
    const base = this.baseUrl.replace(/^http/, 'ws');
    return `${base}/ws/parties/${partyId}`;
  }
}

export const apiClient = new DhunApiClient();
