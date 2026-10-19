// Serves the data pack out of R2 under its etag and pairs two players in a Go Online room
import { DurableObject } from 'cloudflare:workers';

const PACK = 'crunch.pak';
const IMMUTABLE = 'public, max-age=31536000, immutable';
const CODE_ALPHABET = 'ABCDEFGHJKLMNPQRSTUVWXYZ';
const CODE_LEN = 4;
const CODE_TRIES = 8;
const TAG_CODE = 0x80;
const TAG_PEER_LEFT = 0x81;
const CLOSE_NO_SUCH_GAME = 4404;
const CLOSE_GAME_FULL = 4409;
const CLOSE_HOST_LEFT = 4410;
const CLOSE_EXPIRED = 4408;
const ROOM_LIFETIME_MS = 6 * 60 * 60 * 1000;
const MAX_MESSAGE_BYTES = 4096;

function status(code, text) {
  return new Response(text, { status: code, headers: { 'content-type': 'text/plain' } });
}

function randomCode() {
  const bytes = crypto.getRandomValues(new Uint8Array(CODE_LEN));
  return Array.from(bytes, (b) => CODE_ALPHABET[b % CODE_ALPHABET.length]).join('');
}

// Accepts a socket only to close it with a reason the game can show
function refuse(code, reason) {
  const pair = new WebSocketPair();
  const [client, server] = Object.values(pair);
  server.accept();
  server.close(code, reason);
  return new Response(null, { status: 101, webSocket: client });
}

// One room keyed by its code, relaying frames between the host and the single helper
export class Room extends DurableObject {
  async fetch(request) {
    if (request.headers.get('upgrade')?.toLowerCase() !== 'websocket') return status(426, 'websocket required');
    const url = new URL(request.url);
    const role = url.searchParams.get('role');
    const code = url.searchParams.get('code') ?? '';
    const hosts = this.ctx.getWebSockets('host');
    const joiners = this.ctx.getWebSockets('joiner');
    if (role === 'host') {
      if (hosts.length) return status(409, 'code in use');
    } else if (role === 'joiner') {
      if (!hosts.length) return refuse(CLOSE_NO_SUCH_GAME, 'no such game');
      if (joiners.length) return refuse(CLOSE_GAME_FULL, 'game is full');
    } else {
      return status(400, 'role must be host or joiner');
    }
    const pair = new WebSocketPair();
    const [client, server] = Object.values(pair);
    this.ctx.acceptWebSocket(server, [role]);
    server.serializeAttachment({ role });
    if (role === 'host') {
      server.send(new Uint8Array([TAG_CODE, ...new TextEncoder().encode(code)]));
      await this.ctx.storage.setAlarm(Date.now() + ROOM_LIFETIME_MS);
    }
    return new Response(null, { status: 101, webSocket: client });
  }

  role(ws) {
    return ws.deserializeAttachment()?.role;
  }

  others(ws) {
    return this.ctx.getWebSockets(this.role(ws) === 'host' ? 'joiner' : 'host');
  }

  webSocketMessage(ws, message) {
    if (typeof message === 'string' || message.byteLength > MAX_MESSAGE_BYTES) return;
    for (const other of this.others(ws)) other.send(message);
  }

  webSocketClose(ws, code, reason) {
    this.peerGone(ws);
    ws.close(code, reason);
  }

  webSocketError(ws) {
    this.peerGone(ws);
  }

  // Closes the helper when the host goes and tells the host when the helper goes
  peerGone(ws) {
    const role = this.role(ws);
    for (const other of this.others(ws)) {
      other.send(new Uint8Array([TAG_PEER_LEFT]));
      if (role === 'host') setTimeout(() => other.close(CLOSE_HOST_LEFT, 'host left'), 0);
    }
  }

  async alarm() {
    for (const ws of this.ctx.getWebSockets()) ws.close(CLOSE_EXPIRED, 'room expired');
  }
}

// Picks a free code and opens its room for the host
async function host(request, env) {
  for (let i = 0; i < CODE_TRIES; i++) {
    const code = randomCode();
    const url = new URL(request.url);
    url.search = `?role=host&code=${code}`;
    const response = await env.ROOMS.getByName(code).fetch(new Request(url, request));
    if (response.status !== 409) return response;
  }
  return status(503, 'no free game code');
}

async function join(request, env, code) {
  if (code.length !== CODE_LEN || ![...code].every((c) => CODE_ALPHABET.includes(c))) {
    return refuse(CLOSE_NO_SUCH_GAME, 'no such game');
  }
  const url = new URL(request.url);
  url.search = `?role=joiner&code=${code}`;
  return env.ROOMS.getByName(code).fetch(new Request(url, request));
}

async function online(request, env, parts) {
  if (request.headers.get('upgrade')?.toLowerCase() !== 'websocket') return status(426, 'websocket required');
  if (parts.length === 3 && parts[2] === 'host') return host(request, env);
  if (parts.length === 4 && parts[2] === 'join') return join(request, env, parts[3].toUpperCase());
  return status(404, 'not found');
}

async function data(env, parts) {
  if (parts.length === 3 && parts[2] === 'version') {
    const head = await env.DATA.head(PACK);
    if (!head) return status(503, `${PACK} is not in the bucket`);
    return new Response(head.etag, { headers: { 'content-type': 'text/plain', 'cache-control': 'no-store' } });
  }
  if (parts.length !== 4 || parts[3] !== PACK) return status(404, 'not found');
  const object = await env.DATA.get(PACK);
  if (!object) return status(503, `${PACK} is not in the bucket`);
  if (object.etag !== parts[2]) {
    await object.body.cancel();
    return status(404, `pack ${parts[2]} is no longer current`);
  }
  return new Response(object.body, {
    headers: {
      'content-type': 'application/octet-stream',
      'content-length': String(object.size),
      'cache-control': IMMUTABLE,
      etag: object.httpEtag,
    },
  });
}

export default {
  async fetch(request, env) {
    if (request.method !== 'GET') return status(405, 'method not allowed');
    const parts = new URL(request.url).pathname.split('/');
    if (parts[1] === 'online') return online(request, env, parts);
    if (parts[1] === 'data') return data(env, parts);
    return status(404, 'not found');
  },
};
