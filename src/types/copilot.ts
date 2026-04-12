export interface CopilotStatus {
  authenticated: boolean;
  pending: boolean;
  account_type: string | null;
  account_login: string | null;
  has_token: boolean;
  token_expires_at: string | null;
}

export interface CopilotAccountStatus {
  id: number;
  provider_id: string;
  github_login: string;
  avatar_url: string | null;
  account_type: string;
  authenticated: boolean;
  token_expires_at: string | null;
  error: string | null;
}

export interface DeviceCodeResponse {
  device_code: string;
  user_code: string;
  verification_uri: string;
  expires_in: number;
  interval: number;
}
