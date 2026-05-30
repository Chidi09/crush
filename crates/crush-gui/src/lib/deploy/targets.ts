// Deploy target catalog — the per-provider knowledge (config templates, exact
// CLI commands, token env vars) lives here so the backend stays a thin, generic
// "write file + run allow-listed CLI" layer. Grounded in the provider research.
//
// model 'build'    → platform builds from the Dockerfile in the repo.
// model 'registry' → needs a prebuilt OCI image pushed to the provider's
//                     registry first (auto-run disabled; we show the command).

export interface DeployStack {
  name: string;
  port: number;
  runtime?: string | null;
  framework?: string | null;
}

export interface DeployField {
  key: string;
  label: string;
  placeholder?: string;
  secret?: boolean;
  def?: (s: DeployStack) => string;
}

export type StackKind = 'frontend' | 'backend' | 'fullstack';

export interface DeployTarget {
  id: string;
  label: string;
  group: 'build' | 'registry' | 'frontend' | 'vps';
  /** which stack kinds this target is a good fit for (drives recommendations) */
  suits: ('frontend' | 'backend')[];
  /** TechIcon slug for the brand logo; '' falls back to a monogram */
  icon: string;
  /** monogram + accent used when there's no logo (AWS/Azure) */
  mono?: string;
  accent?: string;
  cli: string;
  cliProbe: string;
  install: string;
  tokenEnv: string;
  tokenLabel: string;
  /** interactive login command to run in a popped terminal (browser auth) */
  auth?: string;
  needsDockerfile: boolean;
  autoRun: boolean;
  configFile?: string;
  fields: DeployField[];
  /** generated provider-native config content (null = none) */
  config?: (s: DeployStack, f: Record<string, string>) => string;
  /** the deploy command (program + args). null = manual only (no clean CLI) */
  command?: (s: DeployStack, f: Record<string, string>) => { program: string; args: string[] } | null;
  /** env vars for the child process, built from the entered token + fields */
  env?: (token: string, f: Record<string, string>) => Record<string, string>;
  /** how the public URL is obtained (shown to the user) */
  urlHint?: string;
  notes?: string;
}

const J = (o: unknown) => JSON.stringify(o, null, 2);

export const DEPLOY_TARGETS: DeployTarget[] = [
  {
    id: 'railway',
    auth: 'railway login',
    suits: ['backend'],
    icon: 'railway',
    label: 'Railway',
    group: 'build',
    cli: 'railway',
    cliProbe: '--version',
    install: 'npm i -g @railway/cli',
    tokenEnv: 'RAILWAY_TOKEN',
    tokenLabel: 'Railway project token',
    needsDockerfile: true,
    autoRun: true,
    configFile: 'railway.json',
    fields: [
      { key: 'service', label: 'Service name', def: (s) => s.name },
      { key: 'environment', label: 'Environment', def: () => 'production' },
    ],
    config: () => J({
      $schema: 'https://railway.app/railway.schema.json',
      build: { builder: 'DOCKERFILE', dockerfilePath: './Dockerfile' },
    }),
    command: (_s, f) => ({
      program: 'railway',
      args: ['up', '--service', f.service, '--environment', f.environment, '--detach'],
    }),
    env: (token) => ({ RAILWAY_TOKEN: token }),
    urlHint: 'railway domain  (or parse `railway status --json`)',
  },
  {
    id: 'fly',
    auth: 'flyctl auth login',
    suits: ['backend'],
    icon: 'fly',
    label: 'Fly.io',
    group: 'build',
    cli: 'flyctl',
    cliProbe: 'version',
    install: 'curl -L https://fly.io/install.sh | sh',
    tokenEnv: 'FLY_API_TOKEN',
    tokenLabel: 'Fly API token',
    needsDockerfile: true,
    autoRun: true,
    configFile: 'fly.toml',
    fields: [
      { key: 'app', label: 'App name', def: (s) => `crush-${s.name}` },
      { key: 'region', label: 'Primary region', def: () => 'ord' },
    ],
    config: (s, f) =>
`app = "${f.app}"
primary_region = "${f.region}"

[build]
  dockerfile = "Dockerfile"

[http_service]
  internal_port = ${s.port}
  force_https = true
  auto_stop_machines = true
  auto_start_machines = true
  min_machines_running = 0
`,
    command: (_s, f) => ({ program: 'flyctl', args: ['deploy', '--remote-only', '--app', f.app] }),
    env: (token) => ({ FLY_API_TOKEN: token }),
    urlHint: 'flyctl status --json  →  .Hostname',
  },
  {
    id: 'digitalocean',
    auth: 'doctl auth init',
    suits: ['backend', 'frontend'],
    icon: 'digitalocean',
    label: 'DigitalOcean App Platform',
    group: 'build',
    cli: 'doctl',
    cliProbe: 'version',
    install: 'https://docs.digitalocean.com/reference/doctl/how-to/install/',
    tokenEnv: 'DIGITALOCEAN_ACCESS_TOKEN',
    tokenLabel: 'DigitalOcean API token',
    needsDockerfile: true,
    autoRun: true,
    configFile: '.do/app.yaml',
    fields: [
      { key: 'appName', label: 'App name', def: (s) => s.name },
      { key: 'region', label: 'Region', def: () => 'nyc' },
      { key: 'size', label: 'Instance size', def: () => 'basic-xxs' },
    ],
    config: (s, f) =>
`name: ${f.appName}
region: ${f.region}
services:
  - name: web
    dockerfile_path: Dockerfile
    source_dir: /
    http_port: ${s.port}
    instance_count: 1
    instance_size_slug: ${f.size}
`,
    command: () => ({ program: 'doctl', args: ['apps', 'create', '--spec', '.do/app.yaml', '--format', 'ID', '--no-header'] }),
    env: (token) => ({ DIGITALOCEAN_ACCESS_TOKEN: token }),
    urlHint: 'doctl apps get <id> -o json  →  .live_url',
    notes: 'App Platform builds from a connected Git repo. You may need to add a `github:`/`git:` block to .do/app.yaml or connect the repo in the DO dashboard.',
  },
  {
    id: 'render',
    suits: ['backend', 'frontend'],
    icon: 'render',
    label: 'Render',
    group: 'build',
    cli: 'curl',
    cliProbe: '--version',
    install: '(no CLI — deploys via Git Blueprint)',
    tokenEnv: 'RENDER_API_KEY',
    tokenLabel: 'Render API key',
    needsDockerfile: true,
    autoRun: false,
    configFile: 'render.yaml',
    fields: [
      { key: 'name', label: 'Service name', def: (s) => s.name },
      { key: 'region', label: 'Region', def: () => 'oregon' },
      { key: 'plan', label: 'Plan', def: () => 'free' },
    ],
    config: (_s, f) =>
`services:
  - type: web
    name: ${f.name}
    runtime: docker
    dockerfilePath: ./Dockerfile
    region: ${f.region}
    plan: ${f.plan}
`,
    command: () => null,
    urlHint: 'service.url in the Render API / dashboard',
    notes: 'Commit render.yaml, then on Render: New + → Blueprint → connect this repo. Render auto-deploys on push (no standalone deploy CLI).',
  },
  {
    id: 'cloudrun',
    auth: 'gcloud auth login',
    suits: ['backend'],
    icon: 'googlecloud',
    label: 'GCP Cloud Run',
    group: 'registry',
    cli: 'gcloud',
    cliProbe: 'version',
    install: 'https://cloud.google.com/sdk/docs/install',
    tokenEnv: 'GOOGLE_APPLICATION_CREDENTIALS',
    tokenLabel: 'Path to service-account JSON key',
    needsDockerfile: true,
    autoRun: false,
    fields: [
      { key: 'service', label: 'Service name', def: (s) => s.name },
      { key: 'region', label: 'Region', def: () => 'us-central1' },
      { key: 'image', label: 'Image URI (Artifact Registry)', placeholder: 'REGION-docker.pkg.dev/PROJECT/REPO/IMG:tag' },
    ],
    command: (s, f) => ({
      program: 'gcloud',
      args: ['run', 'deploy', f.service, '--image', f.image || '<IMAGE_URI>', '--region', f.region, '--allow-unauthenticated', '--port', String(s.port), '--quiet'],
    }),
    env: () => ({}),
    urlHint: 'gcloud run services describe <svc> --region <r> --format json  →  .status.url',
    notes: 'Build & push the image to Artifact Registry first (docker push REGION-docker.pkg.dev/…). Then run the command.',
  },
  {
    id: 'apprunner',
    auth: 'aws configure',
    suits: ['backend'],
    icon: '', mono: 'AWS', accent: '#FF9900',
    label: 'AWS App Runner',
    group: 'registry',
    cli: 'aws',
    cliProbe: '--version',
    install: 'https://docs.aws.amazon.com/cli/latest/userguide/getting-started-install.html',
    tokenEnv: 'AWS_ACCESS_KEY_ID',
    tokenLabel: 'AWS_ACCESS_KEY_ID (set SECRET/REGION in env too)',
    needsDockerfile: true,
    autoRun: false,
    configFile: 'apprunner.json',
    fields: [
      { key: 'service', label: 'Service name', def: (s) => s.name },
      { key: 'region', label: 'Region', def: () => 'us-east-1' },
      { key: 'image', label: 'ECR image URI', placeholder: 'ACCT.dkr.ecr.REGION.amazonaws.com/REPO:tag' },
    ],
    config: (s, f) => J({
      SourceConfiguration: {
        ImageRepository: {
          ImageIdentifier: f.image || '<ECR_IMAGE_URI>',
          ImageRepositoryType: 'ECR',
          ImageConfiguration: { Port: String(s.port) },
        },
      },
    }),
    command: (_s, f) => ({
      program: 'aws',
      args: ['apprunner', 'create-service', '--service-name', f.service, '--region', f.region, '--source-configuration', 'file://apprunner.json'],
    }),
    env: () => ({}),
    urlHint: 'aws apprunner describe-service --service-arn …  →  Service.ServiceUrl',
    notes: 'Push the image to ECR first. App Runner needs an access-role ARN to read ECR — add AuthenticationConfiguration.AccessRoleArn to apprunner.json.',
  },
  {
    id: 'azure',
    auth: 'az login',
    suits: ['backend'],
    icon: '', mono: 'AZ', accent: '#0078D4',
    label: 'Azure Container Apps',
    group: 'build',
    cli: 'az',
    cliProbe: 'version',
    install: 'https://learn.microsoft.com/cli/azure/install-azure-cli',
    tokenEnv: 'AZURE_CLIENT_ID',
    tokenLabel: 'Service principal (AZURE_CLIENT_ID; set TENANT/SECRET in env)',
    needsDockerfile: true,
    autoRun: false,
    fields: [
      { key: 'app', label: 'App name', def: (s) => s.name },
      { key: 'resourceGroup', label: 'Resource group', placeholder: 'my-rg' },
      { key: 'environment', label: 'Container Apps environment', placeholder: 'my-env' },
    ],
    command: (s, f) => ({
      program: 'az',
      args: ['containerapp', 'up', '--name', f.app, '--resource-group', f.resourceGroup || '<RG>', '--environment', f.environment || '<ENV>', '--source', '.', '--ingress', 'external', '--target-port', String(s.port)],
    }),
    env: () => ({}),
    urlHint: 'add  --query properties.configuration.ingress.fqdn -o tsv',
    notes: 'Run `az login --service-principal` with your AZURE_* creds first. `containerapp up` builds from source (no registry push needed).',
  },
  {
    id: 'vercel',
    auth: 'vercel login',
    suits: ['frontend'],
    icon: 'vercel',
    label: 'Vercel',
    group: 'frontend',
    cli: 'vercel',
    cliProbe: '--version',
    install: 'npm i -g vercel',
    tokenEnv: 'VERCEL_TOKEN',
    tokenLabel: 'Vercel token',
    needsDockerfile: false,
    autoRun: true,
    fields: [
      { key: 'orgId', label: 'Org ID (optional, for CI)', placeholder: 'team_…' },
      { key: 'projectId', label: 'Project ID (optional, for CI)', placeholder: 'prj_…' },
    ],
    command: () => ({ program: 'vercel', args: ['deploy', '--prod', '--yes'] }),
    env: (token, f) => {
      const e: Record<string, string> = { VERCEL_TOKEN: token };
      if (f.orgId) e.VERCEL_ORG_ID = f.orgId;
      if (f.projectId) e.VERCEL_PROJECT_ID = f.projectId;
      return e;
    },
    urlHint: 'printed to stdout on `vercel deploy`',
    notes: 'Vercel auto-detects the framework (Next/Vite/etc.) — no Dockerfile. First run may prompt to link; set Org/Project ID for fully non-interactive CI.',
  },
  {
    id: 'netlify',
    auth: 'netlify login',
    suits: ['frontend'],
    icon: 'netlify',
    label: 'Netlify',
    group: 'frontend',
    cli: 'netlify',
    cliProbe: '--version',
    install: 'npm i -g netlify-cli',
    tokenEnv: 'NETLIFY_AUTH_TOKEN',
    tokenLabel: 'Netlify auth token',
    needsDockerfile: false,
    autoRun: true,
    configFile: 'netlify.toml',
    fields: [
      { key: 'publish', label: 'Publish directory', def: () => 'dist' },
      { key: 'siteId', label: 'Site ID (optional)', placeholder: 'site_…' },
    ],
    config: (_s, f) =>
`[build]
  publish = "${f.publish || 'dist'}"
`,
    command: (_s, f) => ({ program: 'netlify', args: ['deploy', '--prod', '--dir', f.publish || 'dist', '--json'] }),
    env: (token, f) => {
      const e: Record<string, string> = { NETLIFY_AUTH_TOKEN: token };
      if (f.siteId) e.NETLIFY_SITE_ID = f.siteId;
      return e;
    },
    urlHint: 'deploy_url / ssl_url in the --json output',
  },
  {
    id: 'hetzner-vps',
    suits: ['backend'],
    icon: 'hetzner',
    label: 'Hetzner VPS (Docker + Caddy)',
    group: 'vps',
    cli: 'hcloud',
    cliProbe: 'version',
    install: 'https://github.com/hetznercloud/cli  (HCLOUD_TOKEN env for create)',
    tokenEnv: 'HCLOUD_TOKEN',
    tokenLabel: 'Hetzner API token (only needed to create a new server)',
    needsDockerfile: true,
    autoRun: false, // interactive (SSH host-key, provisioning) → runs in a real terminal
    configFile: 'deploy-hetzner.ps1',
    fields: [
      { key: 'host', label: 'Existing server IP (blank = create one)', placeholder: 'leave blank to provision' },
      { key: 'domain', label: 'Domain (for auto-TLS via Caddy)', placeholder: 'app.example.com' },
      { key: 'sshUser', label: 'SSH user', def: () => 'root' },
      { key: 'serverName', label: 'Server name (when creating)', def: (s) => `crush-${s.name}` },
      { key: 'serverType', label: 'Server type (when creating)', def: () => 'cpx11' },
      { key: 'sshKeyName', label: 'Hetzner SSH key name (when creating)', placeholder: 'my-key' },
    ],
    // Registry-free VPS deploy: build locally → ship the image over SSH → run on
    // a single-node Swarm behind caddy-docker-proxy (auto-TLS). Reviewable script
    // — Crush generates it like `eject`, you run it in a real terminal.
    config: (s, f) =>
`# ─────────────────────────────────────────────────────────────────────────────
# Crush → Hetzner VPS — registry-free deploy (Docker Swarm + Caddy auto-TLS)
# Review before running.  Requires: docker, ssh (OpenSSH), and hcloud if creating.
# This is a starting point — tweak to taste (it's yours).
# ─────────────────────────────────────────────────────────────────────────────
$ErrorActionPreference = "Stop"

$IMAGE       = "crush-${f.appName || s.name}:latest"
$REMOTE_HOST = "${f.host}"            # existing server IP, or blank to create one
$SERVER_NAME = "${f.serverName}"
$SERVER_TYPE = "${f.serverType}"
$SSH_KEY     = "${f.sshKeyName}"      # hcloud ssh-key name (for create)
$SSH_USER    = "${f.sshUser || 'root'}"
$DOMAIN      = "${f.domain}"
$PORT        = ${s.port}

# 1) Provision a server if none given (needs HCLOUD_TOKEN in your env)
if (-not $REMOTE_HOST) {
  Write-Host "Creating Hetzner server $SERVER_NAME ($SERVER_TYPE, docker-ce)..."
  hcloud server create --name $SERVER_NAME --type $SERVER_TYPE --image docker-ce --ssh-key $SSH_KEY
  $REMOTE_HOST = (hcloud server ip $SERVER_NAME).Trim()
  Write-Host "Server IP: $REMOTE_HOST"
}
$SSH = "$SSH_USER@$REMOTE_HOST"

# 2) Wait for SSH to come up (cloud-init installs Docker — don't race it)
Write-Host "Waiting for SSH on $REMOTE_HOST ..."
do { Start-Sleep 3 } until (Test-NetConnection $REMOTE_HOST -Port 22 -InformationLevel Quiet)

# 3) Build the image locally (Dockerfile from \`crush eject\`)
Write-Host "Building $IMAGE ..."
docker build -t $IMAGE .

# 4) Ship the image WITHOUT a registry. NOTE: PowerShell mangles binary in pipes,
#    so save → scp → load (don't pipe \`docker save | ssh\` from PowerShell).
Write-Host "Shipping image to $REMOTE_HOST (no registry)..."
docker save -o crush-image.tar $IMAGE
scp crush-image.tar "$($SSH):/tmp/crush-image.tar"
ssh $SSH "docker load -i /tmp/crush-image.tar && rm -f /tmp/crush-image.tar"
Remove-Item crush-image.tar

# 5) One-time: single-node Swarm + caddy-docker-proxy (auto-TLS, idempotent)
ssh $SSH @'
docker swarm init 2>/dev/null || true
docker network create --driver overlay --attachable caddy 2>/dev/null || true
docker service inspect caddy >/dev/null 2>&1 || docker service create --name caddy \
  --network caddy --constraint node.role==manager \
  --publish 80:80 --publish 443:443 \
  --mount type=bind,source=/var/run/docker.sock,target=/var/run/docker.sock \
  --mount type=volume,source=caddy_data,target=/data \
  -e CADDY_INGRESS_NETWORKS=caddy \
  lucaslorentz/caddy-docker-proxy:ci-alpine
'@

# 6) Zero-downtime rolling deploy + Caddy label (auto HTTPS for $DOMAIN)
ssh $SSH "docker service rm crush_app 2>/dev/null || true; docker service create --name crush_app --network caddy --label caddy=$DOMAIN --label 'caddy.reverse_proxy={{upstreams $PORT}}' --update-order start-first $IMAGE"

Write-Host ""
Write-Host "Done. https://$DOMAIN will be live once Caddy issues the TLS cert."
`,
    command: () => ({ program: 'powershell', args: ['-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', 'deploy-hetzner.ps1'] }),
    urlHint: 'https://<domain> once Caddy issues the certificate',
    notes: 'Registry-free: builds locally, ships the image over SSH (save→scp→load), runs it on a single-node Docker Swarm behind caddy-docker-proxy for auto-TLS. Generate the script, review it, then Open in terminal (SSH host-key + provisioning prompts run there).',
  },
];

// Classify a detected stack so the wizard recommends the right targets.
// Pure SPA frameworks → frontend; SSR/meta-frameworks → fullstack (both);
// everything else (API frameworks, bare runtimes) → backend.
const FRONTEND = ['vite', 'react', 'vue', 'svelte', 'angular', 'astro', 'solid', 'preact'];
const FULLSTACK = ['next', 'nextjs', 'nuxt', 'nuxtjs', 'remix', 'sveltekit', 'analog'];
export function classifyStack(s: DeployStack): StackKind {
  const fw = (s.framework ?? '').toLowerCase().replace(/[^a-z0-9]/g, '');
  if (FULLSTACK.some((x) => fw.includes(x))) return 'fullstack';
  if (FRONTEND.some((x) => fw.includes(x))) return 'frontend';
  return 'backend';
}

/** Split targets into recommended-for-this-stack vs the rest. */
export function recommendFor(kind: StackKind): { recommended: DeployTarget[]; others: DeployTarget[] } {
  const wants = (t: DeployTarget) =>
    kind === 'fullstack' ? true : t.suits.includes(kind);
  return {
    recommended: DEPLOY_TARGETS.filter(wants),
    others: DEPLOY_TARGETS.filter((t) => !wants(t)),
  };
}

export function findTarget(id: string): DeployTarget | undefined {
  return DEPLOY_TARGETS.find((t) => t.id === id);
}

/** Best-effort: pull the first http(s) URL or known PaaS hostname out of CLI output. */
export function parseDeployUrl(text: string): string | null {
  const host = text.match(/https?:\/\/[^\s"']+/i);
  if (host) return host[0];
  const paas = text.match(/[a-z0-9-]+\.(?:up\.railway\.app|fly\.dev|ondigitalocean\.app|onrender\.com|run\.app|awsapprunner\.com|azurecontainerapps\.io)/i);
  return paas ? `https://${paas[0]}` : null;
}
