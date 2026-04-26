const RECENT_TOOLS_KEY = 'starttools.recentToolIds';
const MAX_RECENT_TOOLS = 100;

function normalizeToolId(toolOrId) {
  const id = typeof toolOrId === 'object' ? toolOrId?.id : toolOrId;
  const parsed = Number(id);
  return Number.isFinite(parsed) && parsed > 0 ? parsed : null;
}

export function getRecentToolIds() {
  try {
    const value = JSON.parse(localStorage.getItem(RECENT_TOOLS_KEY) || '[]');
    return Array.isArray(value) ? value.map(Number).filter((id) => Number.isFinite(id)) : [];
  } catch {
    return [];
  }
}

export function markToolUsed(toolOrId) {
  const id = normalizeToolId(toolOrId);
  if (!id) {
    return;
  }

  const recentIds = getRecentToolIds().filter((item) => item !== id);
  recentIds.unshift(id);
  localStorage.setItem(RECENT_TOOLS_KEY, JSON.stringify(recentIds.slice(0, MAX_RECENT_TOOLS)));
}

export function sortToolsForView(tools, field, type = 'desc') {
  const sorted = [...tools];
  const direction = type === 'asc' ? -1 : 1;

  if (field === 'recent') {
    const recentRank = new Map(getRecentToolIds().map((id, index) => [id, index]));
    sorted.sort((a, b) => {
      const aRank = recentRank.has(Number(a.id)) ? recentRank.get(Number(a.id)) : Number.MAX_SAFE_INTEGER;
      const bRank = recentRank.has(Number(b.id)) ? recentRank.get(Number(b.id)) : Number.MAX_SAFE_INTEGER;
      if (aRank !== bRank) {
        return (aRank - bRank) * direction;
      }
      return String(a.name || '').localeCompare(String(b.name || ''));
    });
    return sorted;
  }

  if (field === 'number') {
    sorted.sort((a, b) => (Number(b.number || 0) - Number(a.number || 0)) * direction);
    return sorted;
  }

  sorted.sort((a, b) => String(a.name || '').localeCompare(String(b.name || '')) * direction);
  return sorted;
}
