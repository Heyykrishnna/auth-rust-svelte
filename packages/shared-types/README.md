# @secure-auth/shared-types

Shared TypeScript domain interfaces, DTOs, and API contract specifications for the `secure-auth-platform` monorepo.

## Overview

This package serves as the single source of truth for TypeScript typings shared across frontend clients, mock servers, and API integration tests:

- **User Domain**: `AuthUser`, `UserProfile`, `UserStatus`
- **Session Domain**: `AuthSession`, `SessionItem`, `ActiveSession`, `AuthTokens`
- **Request DTOs**: `LoginRequest`, `RegisterRequest`, `RefreshRequest`, `ResetPasswordRequest`, etc.
- **Response DTOs**: `ApiResponse<T>`, `MessageResponse`, `HealthResponse`, `AuditLogEntry`
- **RBAC**: `Role`, `Permission`, `UserRole`

## Usage

In `apps/web`:
```ts
import type { AuthUser, LoginRequest, AuthTokens } from '@secure-auth/shared-types';
```
