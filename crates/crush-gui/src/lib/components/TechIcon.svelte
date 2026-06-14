<script lang="ts" module>
  // Brand logos for detected stacks, pulled from the `simple-icons` package
  // (bundled SVG paths + official brand hex). We import only the icons we map,
  // so Vite tree-shakes the rest of the ~3k-icon set out of the bundle.
  // CSP blocks remote images (img-src 'self' data:), so inline SVG is required.
  //
  // Coverage goal: every runtime, framework, DB and service the detector can
  // surface resolves to *something* — a brand icon where Simple Icons has one,
  // otherwise the closest language icon, otherwise a coloured monogram (so an
  // unknown stack still renders a tidy lettered badge instead of nothing).
  import {
    // runtimes
    siNodedotjs, siBun, siDeno, siPython, siGo, siRust, siPhp, siRuby,
    siTypescript, siJavascript, siDocker, siDotnet, siSwift, siElixir,
    // node frameworks
    siFastify, siExpress, siNestjs, siHono, siTrpc,
    siVite, siNextdotjs, siNuxt, siRemix, siReact, siSvelte, siVuedotjs,
    siAstro, siAngular, siQwik, siSolid,
    siTurborepo, siTurbo, siNx, siLerna,
    // python frameworks
    siFastapi, siFlask, siDjango, siAiohttp,
    // rust frameworks
    siActix, siRocket, siWarp, siTide,
    // go frameworks
    siGin,
    // java
    siSpring, siSpringboot, siQuarkus, siApachemaven, siGradle,
    // ruby / php / elixir / swift
    siLaravel, siRubyonrails, siSymfony, siCodeigniter, siCakephp,
    siPhoenixframework, siVapor,
    // extra backend frameworks
    siKoa, siAdonisjs, siStrapi,
    // databases & services
    siPostgresql, siRedis, siMongodb, siMysql, siMariadb, siSqlite,
    siMinio, siApachekafka, siApachecassandra, siClickhouse,
    siElasticsearch, siRabbitmq,
    // BaaS / hosted data / auth / ORM / API
    siSupabase, siFirebase, siPlanetscale, siNeon, siTurso, siUpstash,
    siCockroachlabs, siClerk, siAuth0, siPrisma, siDrizzle,
    siGraphql, siApollographql, siSocketdotio,
    // deploy providers
    siRailway, siFlydotio, siDigitalocean, siRender, siGooglecloud,
    siVercel, siNetlify, siHetzner,
  } from 'simple-icons';

  type SI = { path: string; hex: string; title: string };

  // Simple Icons dropped the Java logo over Oracle's trademark, so there's no
  // `siJava`. Use a custom coffee-cup mark in Java's orange — recognisable as
  // Java without shipping the trademarked Duke/cup-with-steam logo.
  const javaIcon: SI = {
    title: 'Java',
    hex: 'ED8B00',
    path: 'M20 3H4v10c0 2.21 1.79 4 4 4h6c2.21 0 4-1.79 4-4v-3h2c1.11 0 2-.89 2-2V5c0-1.11-.89-2-2-2zm0 5h-2V5h2v3zM4 19h16v2H4z',
  };

  // Detected runtime/framework/service strings → Simple Icons. Keys are
  // lowercased with dots kept and everything else stripped, matching `norm`
  // below — so "Node.js", "FastAPI", "Actix-web", "Spring Boot" all resolve.
  // Where Simple Icons has no logo (trademark removals, niche frameworks), we
  // fall back to the closest language icon so the colour still reads right.
  const MAP: Record<string, SI> = {
    // ── runtimes ──
    node: siNodedotjs, nodejs: siNodedotjs,
    bun: siBun, deno: siDeno,
    python: siPython, py: siPython,
    go: siGo, golang: siGo,
    rust: siRust,
    php: siPhp, ruby: siRuby,
    typescript: siTypescript, ts: siTypescript,
    javascript: siJavascript, js: siJavascript,
    dotnet: siDotnet, '.net': siDotnet,
    swift: siSwift, elixir: siElixir,
    docker: siDocker,

    // ── node frameworks ──
    fastify: siFastify, express: siExpress, expressjs: siExpress,
    nest: siNestjs, nestjs: siNestjs,
    hono: siHono, elysia: siBun,
    koa: siKoa,
    adonis: siAdonisjs, adonisjs: siAdonisjs,
    strapi: siStrapi,
    trpc: siTrpc,
    graphql: siGraphql, apollo: siApollographql,
    socketio: siSocketdotio, 'socket.io': siSocketdotio,
    prisma: siPrisma, drizzle: siDrizzle,
    vite: siVite,
    'next.js': siNextdotjs, next: siNextdotjs, nextjs: siNextdotjs,
    nuxt: siNuxt, nuxtjs: siNuxt,
    remix: siRemix,
    react: siReact, svelte: siSvelte, sveltekit: siSvelte,
    vue: siVuedotjs, vuejs: siVuedotjs,
    astro: siAstro,
    angular: siAngular, analogjs: siAngular,
    qwik: siQwik,
    solid: siSolid, solidstart: siSolid,
    turborepo: siTurborepo, turbo: siTurbo, nx: siNx, lerna: siLerna,

    // ── python frameworks ──
    fastapi: siFastapi, flask: siFlask, django: siDjango,
    aiohttp: siAiohttp,
    tornado: siPython, starlette: siPython, litestar: siPython,
    pythonscript: siPython,

    // ── rust frameworks ──
    'actix-web': siActix, actixweb: siActix, actix: siActix,
    axum: siRust,
    rocket: siRocket, warp: siWarp, tide: siTide,

    // ── go frameworks ──
    gin: siGin,
    echo: siGo, fiber: siGo, chi: siGo, grpc: siGo,

    // ── java / .net ──
    java: javaIcon, jvm: javaIcon,
    spring: siSpring, springboot: siSpringboot,
    quarkus: siQuarkus, micronaut: siSpring,
    javamaven: siApachemaven, javagradle: siGradle,
    'asp.netcore': siDotnet, aspnetcore: siDotnet,

    // ── ruby / php / elixir / swift frameworks ──
    laravel: siLaravel,
    rails: siRubyonrails, rubyonrails: siRubyonrails,
    sinatra: siRuby, hanami: siRuby, grape: siRuby,
    symfony: siSymfony, slim: siPhp,
    codeigniter: siCodeigniter, cakephp: siCakephp,
    phoenix: siPhoenixframework,
    vapor: siVapor,

    // ── databases & services ──
    postgres: siPostgresql, postgresql: siPostgresql,
    redis: siRedis, valkey: siRedis,
    mongodb: siMongodb, mongo: siMongodb,
    mysql: siMysql, mariadb: siMariadb,
    sqlite: siSqlite,
    minio: siMinio, s3: siMinio, bucket: siMinio, storage: siMinio,
    kafka: siApachekafka, apachekafka: siApachekafka,
    cassandra: siApachecassandra,
    clickhouse: siClickhouse,
    elasticsearch: siElasticsearch, elastic: siElasticsearch,
    rabbitmq: siRabbitmq,

    // ── BaaS / hosted data / auth ──
    supabase: siSupabase,
    firebase: siFirebase, firestore: siFirebase,
    planetscale: siPlanetscale,
    neon: siNeon,
    turso: siTurso,
    upstash: siUpstash,
    cockroachdb: siCockroachlabs, cockroach: siCockroachlabs,
    clerk: siClerk, auth0: siAuth0,

    // ── deploy providers ──
    railway: siRailway,
    fly: siFlydotio, flyio: siFlydotio, flydotio: siFlydotio,
    digitalocean: siDigitalocean,
    render: siRender,
    googlecloud: siGooglecloud, gcp: siGooglecloud, cloudrun: siGooglecloud,
    vercel: siVercel, netlify: siNetlify,
    hetzner: siHetzner,
  };

  function norm(name: string): string {
    return name.toLowerCase().replace(/[^a-z0-9.]/g, '');
  }

  export function lookupTech(name: string | null | undefined): SI | null {
    if (!name) return null;
    const key = norm(name);
    return MAP[key] ?? MAP[key.replace(/\./g, '')] ?? null;
  }

  // Deterministic colour for monogram fallbacks (stable per name).
  function monoColor(name: string): string {
    let h = 0;
    for (let i = 0; i < name.length; i++) h = (h * 31 + name.charCodeAt(i)) >>> 0;
    return `hsl(${h % 360} 55% 55%)`;
  }

  // Many brands (Next.js, Bun, Deno, Express, Fastify, Symfony, Actix, Vapor,
  // Angular #0F0F11…) ship a near-black logo that vanishes on our dark theme.
  // When the brand hex is too dark to read, render it in light text instead.
  function brandFill(hex: string): string {
    const r = parseInt(hex.slice(0, 2), 16);
    const g = parseInt(hex.slice(2, 4), 16);
    const b = parseInt(hex.slice(4, 6), 16);
    const lum = 0.2126 * r + 0.7152 * g + 0.0722 * b; // 0–255
    return lum < 42 ? '#ededed' : `#${hex}`;
  }
</script>

<script lang="ts">
  let {
    name,
    size = 14,
    /** when true, render in the brand color; otherwise inherit currentColor */
    brand = true
  }: { name: string | null | undefined; size?: number; brand?: boolean } = $props();

  let icon = $derived(lookupTech(name));
  let initial = $derived((name ?? '?').trim().charAt(0).toUpperCase() || '?');
</script>

{#if icon}
  <svg
    width={size}
    height={size}
    viewBox="0 0 24 24"
    fill={brand ? brandFill(icon.hex) : 'currentColor'}
    role="img"
    aria-label={icon.title}
  >
    <path d={icon.path} />
  </svg>
{:else if name}
  <span
    class="mono-badge"
    style="width:{size}px;height:{size}px;font-size:{Math.round(size * 0.6)}px;background:{brand ? monoColor(name) : 'var(--color-crush-surface)'}"
    aria-label={name}
    title={name}
  >{initial}</span>
{/if}

<style>
  .mono-badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    color: #fff;
    font-family: var(--font-mono);
    font-weight: 700;
    line-height: 1;
    flex-shrink: 0;
  }
</style>
