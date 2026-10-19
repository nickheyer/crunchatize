// Serves the data pack out of R2 under its etag so browsers keep one copy per version
const PACK = 'crunch.pak';
const IMMUTABLE = 'public, max-age=31536000, immutable';

function status(code, text) {
  return new Response(text, { status: code, headers: { 'content-type': 'text/plain' } });
}

export default {
  async fetch(request, env) {
    if (request.method !== 'GET') return status(405, 'method not allowed');
    const parts = new URL(request.url).pathname.split('/');
    if (parts[1] !== 'data') return status(404, 'not found');
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
  },
};
