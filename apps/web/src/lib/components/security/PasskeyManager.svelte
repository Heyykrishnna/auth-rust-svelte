<script lang="ts">
  import Card from "$lib/components/Card.svelte";
  import Button from "$lib/components/Button.svelte";
  import DeleteButton from "$lib/components/DeleteButton.svelte";
  import Badge from "$lib/components/Badge.svelte";
  import Modal from "$lib/components/Modal.svelte";
  import Input from "$lib/components/Input.svelte";
  import { securityApi } from "$lib/api/security";
  import type { PasskeyInfo } from "$lib/api/types";
  import { addToast } from "$lib/components/Toast.svelte";
  import { ApiError } from "$lib/api/client";

  let {
    passkeys = [],
    onChanged,
  }: {
    passkeys: PasskeyInfo[];
    onChanged?: () => void;
  } = $props();

  let registering = $state(false);
  let passkeyName = $state("My Device Passkey");
  let nameModalOpen = $state(false);
  let passkeyToDelete = $state<PasskeyInfo | null>(null);
  let deleting = $state(false);

  function bufferToBase64Url(buffer: ArrayBuffer): string {
    const bytes = new Uint8Array(buffer);
    let binary = "";
    for (let i = 0; i < bytes.byteLength; i++) {
      binary += String.fromCharCode(bytes[i]);
    }
    return btoa(binary)
      .replace(/\+/g, "-")
      .replace(/\//g, "_")
      .replace(/=+$/, "");
  }

  function base64UrlToBuffer(base64url: string): ArrayBuffer {
    const base64 = base64url.replace(/-/g, "+").replace(/_/g, "/");
    const binary = atob(base64);
    const bytes = new Uint8Array(binary.length);
    for (let i = 0; i < binary.length; i++) {
      bytes[i] = binary.charCodeAt(i);
    }
    return bytes.buffer;
  }

  function initiateRegistration() {
    passkeyName = getDefaultDeviceName();
    nameModalOpen = true;
  }

  function getDefaultDeviceName(): string {
    if (typeof navigator === "undefined") return "Primary Passkey";
    const ua = navigator.userAgent;
    if (ua.includes("Macintosh")) return "MacBook Touch ID";
    if (ua.includes("iPhone")) return "iPhone Face ID";
    if (ua.includes("iPad")) return "iPad Touch ID";
    if (ua.includes("Windows")) return "Windows Hello";
    if (ua.includes("Android")) return "Android Biometrics";
    return "FIDO2 Security Key";
  }

  async function executeWebAuthnRegistration() {
    nameModalOpen = false;
    registering = true;

    try {
      const options = await securityApi.getPasskeyOptions();

      let credentialId = "";
      let publicKeySpki = "";

      if (typeof window !== "undefined" && window.PublicKeyCredential && navigator.credentials) {
        const challengeBuffer = base64UrlToBuffer(options.challenge);
        const userIdBuffer = new TextEncoder().encode(options.user_id);

        const publicKeyCredentialCreationOptions = {
          challenge: challengeBuffer,
          rp: {
            name: options.rp_name,
            id: window.location.hostname === "localhost" ? "localhost" : window.location.hostname,
          },
          user: {
            id: userIdBuffer,
            name: options.user_email,
            displayName: options.user_display_name,
          },
          pubKeyCredParams: [
            { alg: -7, type: "public-key" as const },
            { alg: -257, type: "public-key" as const },
          ],
          authenticatorSelection: {
            authenticatorAttachment: "platform" as const,
            userVerification: "preferred" as const,
            residentKey: "preferred" as const,
          },
          timeout: 60000,
          attestation: "none" as const,
        };

        const cred = await navigator.credentials.create({
          publicKey: publicKeyCredentialCreationOptions,
        });

        if (!(cred instanceof PublicKeyCredential) || !(cred.response instanceof AuthenticatorAttestationResponse)) {
          throw new Error("Passkey creation was canceled by the user");
        }

        credentialId = cred.id;
        publicKeySpki = bufferToBase64Url(cred.response.attestationObject);
      } else {
        throw new Error("Passkeys are unavailable in this browser.");
      }

      await securityApi.registerPasskey({
        credential_id: credentialId,
        public_key_spki: publicKeySpki,
        name: passkeyName.trim() || "Passkey",
      });

      addToast("Passkey registered successfully", "success");
      onChanged?.();
    } catch (err) {
      if (err instanceof ApiError) {
        addToast(err.message, "error");
      } else if (err instanceof Error) {
        addToast(err.message, "error");
      } else {
        addToast("Failed to register passkey", "error");
      }
    } finally {
      registering = false;
    }
  }

  async function executeDeletePasskey() {
    if (!passkeyToDelete) return;
    deleting = true;
    try {
      await securityApi.deletePasskey(passkeyToDelete.id);
      addToast("Passkey removed", "success");
      passkeyToDelete = null;
      onChanged?.();
    } catch (err) {
      if (err instanceof ApiError) {
        addToast(err.message, "error");
      } else {
        addToast("Failed to remove passkey", "error");
      }
    } finally {
      deleting = false;
    }
  }

  function formatDate(iso: string | null | undefined): string {
    if (!iso) return "Never used";
    try {
      return new Date(iso).toLocaleDateString(undefined, {
        month: "short",
        day: "numeric",
        year: "numeric",
      });
    } catch {
      return iso;
    }
  }
</script>

<Card
  title="Passkeys & FIDO2 WebAuthn"
  subtitle="Sign in securely using fingerprint, Face ID, Windows Hello, or hardware security keys"
>
  {#snippet headerActions()}
    <Button
      size="sm"
      onclick={initiateRegistration}
      loading={registering}
      disabled={registering}
    >
      Register Passkey
    </Button>
  {/snippet}

  <div class="flex flex-col gap-4">
    {#if passkeys.length === 0}
      <div class="text-center py-8 text-sm text-brand-text-muted flex flex-col items-center gap-2">
        <div class="size-12 rounded-full bg-white/5 border border-white/10 flex items-center justify-center text-brand-text-muted">
          <svg class="size-6" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M21 2l-2 2m-1-1l-3 3m2-2l-3 3m-2-2l-3 3m2-2l-3 3"></path>
            <circle cx="7.5" cy="15.5" r="5.5"></circle>
          </svg>
        </div>
        <span>No passkeys registered yet. Add a passkey for instant, passwordless authentication.</span>
      </div>
    {:else}
      <div class="flex flex-col gap-3">
        {#each passkeys as passkey (passkey.id)}
          <div class="flex items-center justify-between p-4 rounded-xl border border-brand-border bg-brand-surface-card">
            <div class="flex items-center gap-3.5 min-w-0">
              <div class="size-10 rounded-lg bg-brand-lime/10 border border-brand-lime/20 flex items-center justify-center shrink-0 text-brand-lime font-mono text-xs font-bold">
                KEY
              </div>
              <div class="flex flex-col gap-1 min-w-0">
                <div class="flex items-center gap-2">
                  <span class="text-sm font-semibold text-brand-text-primary truncate">{passkey.name}</span>
                  <Badge variant="purple" size="sm">FIDO2</Badge>
                </div>
                <div class="flex items-center gap-2 text-xs text-brand-text-muted">
                  <span>Registered: {formatDate(passkey.created_at)}</span>
                  <span>•</span>
                  <span>Last active: {formatDate(passkey.last_used_at)}</span>
                </div>
              </div>
            </div>

            <div>
              <DeleteButton
                label="Delete passkey"
                onclick={() => (passkeyToDelete = passkey)}
              />
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</Card>

<Modal
  open={nameModalOpen}
  title="Name Your Passkey"
  description="Assign a recognizable name to this security credential (e.g. MacBook Touch ID, YubiKey 5)."
  confirmText="Continue with WebAuthn"
  onconfirm={executeWebAuthnRegistration}
  oncancel={() => (nameModalOpen = false)}
>
  <div class="flex flex-col gap-3 py-2">
    <Input
      id="passkey-label"
      label="Passkey Nickname"
      bind:value={passkeyName}
      required
    />
  </div>
</Modal>

<Modal
  open={passkeyToDelete !== null}
  title="Delete Passkey?"
  description="Are you sure you want to remove this passkey? You will no longer be able to use this credential to authenticate."
  confirmText="Delete Passkey"
  variant="danger"
  loading={deleting}
  onconfirm={executeDeletePasskey}
  oncancel={() => (passkeyToDelete = null)}
/>
