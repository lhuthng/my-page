const VIEW_KEY = "viewed_posts";
const LIKE_KEY = "liked_posts";
export const VIEW_DELAY = 3 * 1000; // ms
const COOLDOWN = 5 * 60 * 1000; // ms

function saveViewed(map, postId) {
  if (!map) return false;
  map[postId] = Date.now();
  localStorage.setItem(VIEW_KEY, JSON.stringify(map));
  return true;
}

export function shouldSendView(postId) {
  try {
    const raw = localStorage.getItem(VIEW_KEY);
    if (!raw) {
      return saveViewed({}, postId);
    }

    const map = JSON.parse(String(raw));
    const last = map[String(postId)];
    if (!last) {
      return saveViewed(map, postId);
    }

    if (Date.now() - last > COOLDOWN) {
      return saveViewed(map, postId);
    }
  } catch (e) {
    console.error(e);
    localStorage.setItem(VIEW_KEY, "{}");
  }
  return false;
}

function saveLiked(set, postId) {
  if (!set) return false;
  set.add(postId);
  localStorage.setItem(LIKE_KEY, JSON.stringify(Array.from(set)));
  return true;
}

export function isLiked(postId) {
  try {
    const raw = localStorage.getItem(LIKE_KEY);
    if (!raw) {
      return false;
    }

    const arr = JSON.parse(String(raw));
    const set = new Set(Array.isArray(arr) ? arr : []);

    return set.has(postId);
  } catch (e) {
    console.error(e);
    localStorage.setItem(LIKE_KEY, "[]");
  }
  return false;
}

export function sendLike(postId) {
  try {
    const raw = localStorage.getItem(LIKE_KEY);
    if (!raw) {
      return saveLiked(new Set(), postId);
    }

    const arr = JSON.parse(String(raw));
    const set = new Set(Array.isArray(arr) ? arr : []);

    if (!set.has(postId)) {
      return saveLiked(set, postId);
    }
  } catch (e) {
    console.error(e);
    localStorage.setItem(LIKE_KEY, "[]");
  }
  return false;
}
