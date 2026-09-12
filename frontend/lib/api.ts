import {
  AlertEntity,
  ApiResponse,
  EventLogEntity,
  IsolateNodeResponse,
  LoginResponseData,
  NodeDetailDto,
  NodeSummaryDto,
  PaginatedResult,
  UpdateAlertStatusResponse,
  UserInfo,
} from './types';

const AUTH_KEY = 'aigis_auth';
const USER_KEY = 'aigis_user';

function setAuthCookie(token: string, days = 7): void {
  if (typeof document === 'undefined') return;
  const expires = new Date(Date.now() + days * 864e5).toUTCString();
  document.cookie = `${AUTH_KEY}=${encodeURIComponent(token)}; expires=${expires}; path=/; SameSite=Lax`;
}

function deleteAuthCookie(): void {
  if (typeof document === 'undefined') return;
  document.cookie = `${AUTH_KEY}=; expires=Thu, 01 Jan 1970 00:00:00 GMT; path=/; SameSite=Lax`;
}

export function getAuthToken(): string | null {
  if (typeof window === 'undefined') return null;
  const token = localStorage.getItem(AUTH_KEY);
  if (token && typeof document !== 'undefined' && !document.cookie.includes(AUTH_KEY)) {
    setAuthCookie(token);
  }
  return token;
}

export function setAuthSession(token: string, user: UserInfo): void {
  if (typeof window === 'undefined') return;
  localStorage.setItem(AUTH_KEY, token);
  localStorage.setItem(USER_KEY, JSON.stringify(user));
  setAuthCookie(token);
}

export function clearAuthSession(): void {
  if (typeof window === 'undefined') return;
  localStorage.removeItem(AUTH_KEY);
  localStorage.removeItem(USER_KEY);
  deleteAuthCookie();
}

export function getStoredUser(): UserInfo | null {
  if (typeof window === 'undefined') return null;
  const raw = localStorage.getItem(USER_KEY);
  if (!raw) return null;
  try {
    return JSON.parse(raw) as UserInfo;
  } catch {
    return null;
  }
}

export class ApiError extends Error {
  status: number;
  code?: string;

  constructor(message: string, status: number, code?: string) {
    super(message);
    this.name = 'ApiError';
    this.status = status;
    this.code = code;
  }
}

async function request<T>(path: string, options: RequestInit = {}): Promise<T> {
  const token = getAuthToken();
  const headers = new Headers(options.headers);

  if (!headers.has('Content-Type') && !(options.body instanceof FormData)) {
    headers.set('Content-Type', 'application/json');
  }

  if (token && !headers.has('Authorization')) {
    headers.set('Authorization', `Bearer ${token}`);
  }

  const response = await fetch(path, {
    ...options,
    headers,
  });

  if (response.status === 401) {
    if (typeof window !== 'undefined' && !window.location.pathname.startsWith('/login')) {
      clearAuthSession();
      window.location.href = '/login';
    }
  }

  const text = await response.text();
  let json: unknown;
  try {
    json = text ? JSON.parse(text) : {};
  } catch {
    json = { message: text };
  }

  if (!response.ok) {
    const errObj = json as { error?: { message?: string; code?: string }; message?: string };
    const errMsg = errObj.error?.message || errObj.message || `Request failed with status ${response.status}`;
    throw new ApiError(errMsg, response.status, errObj.error?.code);
  }

  return json as T;
}

export async function login(username: string, password: string): Promise<LoginResponseData> {
  const result = await request<ApiResponse<LoginResponseData>>('/api/v1/auth/login', {
    method: 'POST',
    body: JSON.stringify({ username, password }),
  });
  setAuthSession(result.data.token, result.data.user);
  return result.data;
}

export async function getMe(): Promise<UserInfo> {
  const result = await request<ApiResponse<UserInfo>>('/api/v1/auth/me');
  if (typeof window !== 'undefined' && result.data) {
    localStorage.setItem(USER_KEY, JSON.stringify(result.data));
  }
  return result.data;
}

export async function getNodes(params?: {
  search?: string;
  agent_status?: string;
  operator_status?: string;
  limit?: number;
  offset?: number;
}): Promise<NodeSummaryDto[]> {
  const query = new URLSearchParams();
  if (params?.search) query.set('search', params.search);
  if (params?.agent_status) query.set('agent_status', params.agent_status);
  if (params?.operator_status) query.set('operator_status', params.operator_status);
  if (params?.limit) query.set('limit', params.limit.toString());
  if (params?.offset) query.set('offset', params.offset.toString());

  const queryString = query.toString();
  const path = queryString ? `/api/v1/nodes?${queryString}` : '/api/v1/nodes';
  const res = await request<ApiResponse<PaginatedResult<NodeSummaryDto> | NodeSummaryDto[]>>(path);

  if (Array.isArray(res.data)) {
    return res.data;
  }
  if (res.data && Array.isArray((res.data as PaginatedResult<NodeSummaryDto>).items)) {
    return (res.data as PaginatedResult<NodeSummaryDto>).items;
  }
  return [];
}

export async function getNodeById(nodeId: string): Promise<NodeDetailDto> {
  const res = await request<ApiResponse<NodeDetailDto>>(`/api/v1/nodes/${nodeId}`);
  return res.data;
}

export async function isolateNode(nodeId: string, reason?: string): Promise<IsolateNodeResponse> {
  const res = await request<ApiResponse<IsolateNodeResponse>>(`/api/v1/nodes/${nodeId}/isolate`, {
    method: 'POST',
    body: JSON.stringify({ reason }),
  });
  return res.data;
}

export async function unisolateNode(nodeId: string): Promise<IsolateNodeResponse> {
  const res = await request<ApiResponse<IsolateNodeResponse>>(`/api/v1/nodes/${nodeId}/unisolate`, {
    method: 'POST',
  });
  return res.data;
}

export async function getAlerts(params?: {
  severity?: string;
  status?: string;
  node_id?: string;
  mitre_technique?: string;
  limit?: number;
  offset?: number;
}): Promise<AlertEntity[]> {
  const query = new URLSearchParams();
  if (params?.severity && params.severity !== 'all') query.set('severity', params.severity);
  if (params?.status && params.status !== 'all') query.set('status', params.status);
  if (params?.node_id) query.set('node_id', params.node_id);
  if (params?.mitre_technique) query.set('mitre_technique', params.mitre_technique);
  if (params?.limit) query.set('limit', params.limit.toString());
  if (params?.offset) query.set('offset', params.offset.toString());

  const queryString = query.toString();
  const path = queryString ? `/api/v1/alerts?${queryString}` : '/api/v1/alerts';
  const res = await request<ApiResponse<PaginatedResult<AlertEntity> | AlertEntity[]>>(path);

  if (Array.isArray(res.data)) {
    return res.data;
  }
  if (res.data && Array.isArray((res.data as PaginatedResult<AlertEntity>).items)) {
    return (res.data as PaginatedResult<AlertEntity>).items;
  }
  return [];
}

export async function getAlertById(alertId: string): Promise<AlertEntity> {
  const res = await request<ApiResponse<AlertEntity>>(`/api/v1/alerts/${alertId}`);
  return res.data;
}

export async function updateAlertStatus(
  alertId: string,
  status: string,
  comment?: string
): Promise<UpdateAlertStatusResponse> {
  const res = await request<ApiResponse<UpdateAlertStatusResponse>>(`/api/v1/alerts/${alertId}/status`, {
    method: 'PATCH',
    body: JSON.stringify({ status, comment }),
  });
  return res.data;
}

export async function getLogs(params?: {
  event_type?: string;
  node_id?: string;
  limit?: number;
  offset?: number;
  from_timestamp?: string;
  to_timestamp?: string;
}): Promise<EventLogEntity[]> {
  const query = new URLSearchParams();
  if (params?.event_type && params.event_type !== 'all') query.set('event_type', params.event_type);
  if (params?.node_id) query.set('node_id', params.node_id);
  if (params?.limit) query.set('limit', params.limit.toString());
  if (params?.offset) query.set('offset', params.offset.toString());
  if (params?.from_timestamp) query.set('from_timestamp', params.from_timestamp);
  if (params?.to_timestamp) query.set('to_timestamp', params.to_timestamp);

  const queryString = query.toString();
  const path = queryString ? `/api/v1/logs?${queryString}` : '/api/v1/logs';
  const res = await request<ApiResponse<PaginatedResult<EventLogEntity> | EventLogEntity[]>>(path);

  if (Array.isArray(res.data)) {
    return res.data;
  }
  if (res.data && Array.isArray((res.data as PaginatedResult<EventLogEntity>).items)) {
    return (res.data as PaginatedResult<EventLogEntity>).items;
  }
  return [];
}

export async function getLogById(eventId: string): Promise<EventLogEntity> {
  const res = await request<ApiResponse<EventLogEntity>>(`/api/v1/logs/${eventId}`);
  return res.data;
}

export async function getNodesPaginated(params?: {
  search?: string;
  agent_status?: string;
  operator_status?: string;
  limit?: number;
  offset?: number;
}): Promise<PaginatedResult<NodeSummaryDto>> {
  const query = new URLSearchParams();
  if (params?.search) query.set('search', params.search);
  if (params?.agent_status) query.set('agent_status', params.agent_status);
  if (params?.operator_status) query.set('operator_status', params.operator_status);
  if (params?.limit) query.set('limit', params.limit.toString());
  if (params?.offset) query.set('offset', params.offset.toString());

  const queryString = query.toString();
  const path = queryString ? `/api/v1/nodes?${queryString}` : '/api/v1/nodes';
  const res = await request<ApiResponse<PaginatedResult<NodeSummaryDto> | NodeSummaryDto[]>>(path);

  if (Array.isArray(res.data)) {
    return { items: res.data, total: res.data.length };
  }
  if (res.data && Array.isArray((res.data as PaginatedResult<NodeSummaryDto>).items)) {
    return res.data as PaginatedResult<NodeSummaryDto>;
  }
  return { items: [], total: 0 };
}

export async function getAlertsPaginated(params?: {
  severity?: string;
  status?: string;
  node_id?: string;
  mitre_technique?: string;
  limit?: number;
  offset?: number;
}): Promise<PaginatedResult<AlertEntity>> {
  const query = new URLSearchParams();
  if (params?.severity && params.severity !== 'all') query.set('severity', params.severity);
  if (params?.status && params.status !== 'all') query.set('status', params.status);
  if (params?.node_id) query.set('node_id', params.node_id);
  if (params?.mitre_technique) query.set('mitre_technique', params.mitre_technique);
  if (params?.limit) query.set('limit', params.limit.toString());
  if (params?.offset) query.set('offset', params.offset.toString());

  const queryString = query.toString();
  const path = queryString ? `/api/v1/alerts?${queryString}` : '/api/v1/alerts';
  const res = await request<ApiResponse<PaginatedResult<AlertEntity> | AlertEntity[]>>(path);

  if (Array.isArray(res.data)) {
    return { items: res.data, total: res.data.length };
  }
  if (res.data && Array.isArray((res.data as PaginatedResult<AlertEntity>).items)) {
    return res.data as PaginatedResult<AlertEntity>;
  }
  return { items: [], total: 0 };
}

export async function checkHealth(): Promise<{ status: string }> {
  const res = await request<{ status: string }>('/healthz');
  return res;
}
