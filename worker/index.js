// Serves game files by byte range out of the single pack object in R2, addressed by the pack's etag
const PACK = 'crunch.pak';
const MAGIC = 'CRPK';
const HEADER_LEN = 8;
const IMMUTABLE = 'public, max-age=31536000, immutable';

let index = null;

function status(code, text) {
  return new Response(text, { status: code, headers: { 'content-type': 'text/plain' } });
}

// Returns the parsed index for the requested pack version, reloading it when the object changed
async function loadIndex(env, version) {
  if (index && index.version === version) return index;
  const head = await env.DATA.head(PACK);
  if (!head || head.etag !== version) return null;
  const header = await env.DATA.get(PACK, { range: { offset: 0, length: HEADER_LEN } });
  const bytes = new Uint8Array(await header.arrayBuffer());
  if (new TextDecoder().decode(bytes.subarray(0, 4)) !== MAGIC) throw new Error(`${PACK} is not a CRPK pack`);
  const indexLen = new DataView(bytes.buffer).getUint32(4, true);
  const text = await (await env.DATA.get(PACK, { range: { offset: HEADER_LEN, length: indexLen } })).text();
  const files = new Map();
  for (const line of text.split('\n')) {
    if (!line) continue;
    const [name, offset, size] = line.split('\t');
    files.set(name, { offset: HEADER_LEN + indexLen + Number(offset), size: Number(size) });
  }
  index = { version: head.etag, files };
  return index;
}

export default {
  async fetch(request, env, ctx) {
    if (request.method !== 'GET') return status(405, 'method not allowed');
    const parts = new URL(request.url).pathname.split('/');
    if (parts[1] !== 'data') return status(404, 'not found');
    if (parts.length === 3 && parts[2] === 'version') {
      const head = await env.DATA.head(PACK);
      if (!head) return status(503, `${PACK} is not in the bucket`);
      return new Response(head.etag, { headers: { 'content-type': 'text/plain', 'cache-control': 'no-store' } });
    }
    if (parts.length !== 4) return status(404, 'not found');
    const cache = caches.default;
    const hit = await cache.match(request);
    if (hit) return hit;
    const [, , version, rawName] = parts;
    const name = decodeURIComponent(rawName).toLowerCase();
    const entry = (await loadIndex(env, version))?.files.get(name);
    if (!entry) return status(404, `${name} is not in pack ${version}`);
    const object = await env.DATA.get(PACK, { range: { offset: entry.offset, length: entry.size } });
    if (!object) return status(503, `${PACK} is not in the bucket`);
    const response = new Response(object.body, {
      headers: {
        'content-type': 'application/octet-stream',
        'content-length': String(entry.size),
        'cache-control': IMMUTABLE,
      },
    });
    ctx.waitUntil(cache.put(request, response.clone()));
    return response;
  },
};
