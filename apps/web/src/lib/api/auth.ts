import { apiClient, type RequestOptions } from './client';
import type {
	AuthSession,
	AuthTokens,
	AuthUser,
	ForgotPasswordRequest,
	HealthResponse,
	LoginRequest,
	MessageResponse,
	OidcProvider,
	RefreshRequest,
	RegisterInitiateResponse,
	RegisterRequest,
	RegisterVerifyRequest,
	ResendOtpRequest,
	ResetPasswordRequest,
	SessionItem,
	UpdateProfileRequest,
	VerifyCodeRequest
} from './types';

export * from './types';

export const authApi = {
	register: (data: RegisterRequest, options?: RequestOptions) =>
		options
			? apiClient.post<RegisterInitiateResponse>('/api/auth/register', data, options)
			: apiClient.post<RegisterInitiateResponse>('/api/auth/register', data),

	verifyRegisterOtp: (data: RegisterVerifyRequest, options?: RequestOptions) =>
		options
			? apiClient.post<AuthSession>('/api/auth/register/verify', data, options)
			: apiClient.post<AuthSession>('/api/auth/register/verify', data),

	resendRegisterOtp: (data: ResendOtpRequest, options?: RequestOptions) =>
		options
			? apiClient.post<MessageResponse>('/api/auth/register/resend-otp', data, options)
			: apiClient.post<MessageResponse>('/api/auth/register/resend-otp', data),


	login: (data: LoginRequest, options?: RequestOptions) =>
		options
			? apiClient.post<AuthSession>('/api/auth/login', data, options)
			: apiClient.post<AuthSession>('/api/auth/login', data),

	logout: (options?: RequestOptions) =>
		options
			? apiClient.post<void>('/api/auth/logout', {}, options)
			: apiClient.post<void>('/api/auth/logout', {}),

	refresh: (data?: RefreshRequest, options?: RequestOptions) =>
		options
			? apiClient.post<AuthTokens>('/api/auth/refresh', data ?? {}, options)
			: apiClient.post<AuthTokens>('/api/auth/refresh', data ?? {}),

	me: (options?: RequestOptions) =>
		options
			? apiClient.get<AuthUser>('/api/auth/me', options)
			: apiClient.get<AuthUser>('/api/auth/me'),

	updateProfile: (data: UpdateProfileRequest, options?: RequestOptions) =>
		options
			? apiClient.put<AuthUser>('/api/users/profile', data, options)
			: apiClient.put<AuthUser>('/api/users/profile', data),

	listSessions: (options?: RequestOptions) =>
		options
			? apiClient.get<SessionItem[]>('/api/sessions', options)
			: apiClient.get<SessionItem[]>('/api/sessions'),

	revokeSession: (sessionId: string, options?: RequestOptions) =>
		options
			? apiClient.delete<void>(`/api/sessions/${sessionId}`, options)
			: apiClient.delete<void>(`/api/sessions/${sessionId}`),

	getOidcUrl: (provider: 'google' | 'github', options?: RequestOptions) =>
		options
			? apiClient.get<OidcProvider>(`/api/auth/oidc/${provider}`, options)
			: apiClient.get<OidcProvider>(`/api/auth/oidc/${provider}`),

	oidcCallback: (
		provider: 'google' | 'github',
		code: string,
		state: string,
		options?: RequestOptions
	) =>
		options
			? apiClient.post<AuthSession>(`/api/auth/oidc/${provider}/callback`, { code, state }, options)
			: apiClient.post<AuthSession>(`/api/auth/oidc/${provider}/callback`, { code, state }),

	forgotPassword: (data: ForgotPasswordRequest, options?: RequestOptions) =>
		options
			? apiClient.post<MessageResponse>('/api/auth/forgot-password', data, options)
			: apiClient.post<MessageResponse>('/api/auth/forgot-password', data),

	resetPassword: (data: ResetPasswordRequest, options?: RequestOptions) =>
		options
			? apiClient.post<MessageResponse>('/api/auth/reset-password', data, options)
			: apiClient.post<MessageResponse>('/api/auth/reset-password', data),

	verifyCode: (data: VerifyCodeRequest, options?: RequestOptions) =>
		options
			? apiClient.post<MessageResponse>('/api/auth/verify-code', data, options)
			: apiClient.post<MessageResponse>('/api/auth/verify-code', data),

	health: (options?: RequestOptions) =>
		options
			? apiClient.get<HealthResponse>('/health', options)
			: apiClient.get<HealthResponse>('/health')
};
