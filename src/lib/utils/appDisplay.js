function trimmed(value) {
  return typeof value === 'string' ? value.trim() : '';
}

function normalizeComparable(value) {
  return trimmed(value)
    .toLowerCase()
    .replace(/[^a-z0-9\u4e00-\u9fa5]+/gi, '');
}

function isGenericInstallerToken(value) {
  return ['setup', 'install', 'installer', 'uninstall'].includes(normalizeComparable(value));
}

function isInstallerLikeName(value) {
  const comparable = normalizeComparable(value);
  return (
    comparable.includes('setup')
    || comparable.includes('installer')
    || comparable.includes('uninstall')
    || comparable.includes('install')
  );
}

function isCompactRawToken(value) {
  const source = trimmed(value);
  return Boolean(source) && /^[a-z0-9_.-]+$/i.test(source) && !/[A-Z]/.test(source);
}

export function getPreferredTimelineAppName({ appName, windowTitle }) {
  const rawAppName = trimmed(appName);
  const rawTitle = trimmed(windowTitle);
  if (!rawTitle) {
    return rawAppName;
  }

  const appComparable = normalizeComparable(rawAppName);
  const titleComparable = normalizeComparable(rawTitle);

  if (isGenericInstallerToken(rawAppName) && rawTitle.length > rawAppName.length) {
    return rawTitle;
  }

  if (
    isInstallerLikeName(rawAppName)
    && titleComparable
    && appComparable
    && (
      appComparable.includes(titleComparable)
      || (
        appComparable.startsWith('workreview')
        && titleComparable.startsWith('workreview')
        && appComparable.endsWith('setup')
        && titleComparable.endsWith('setup')
      )
    )
  ) {
    return rawTitle;
  }

  if (appComparable && titleComparable === appComparable && rawTitle !== rawAppName) {
    return rawTitle;
  }

  if (
    isCompactRawToken(rawAppName)
    && titleComparable
    && appComparable
    && (appComparable.includes(titleComparable) || titleComparable.includes(appComparable))
    && rawTitle.length >= rawAppName.length
  ) {
    return rawTitle;
  }

  return rawAppName || rawTitle;
}

export function shouldPreferTimelineFallbackIcon({ appName, windowTitle }) {
  const rawAppName = trimmed(appName);
  const rawTitle = trimmed(windowTitle);
  const preferredName = getPreferredTimelineAppName({ appName: rawAppName, windowTitle: rawTitle });

  if (!preferredName) {
    return false;
  }

  if (isGenericInstallerToken(rawAppName)) {
    return true;
  }

  if (isInstallerLikeName(rawAppName) && rawTitle) {
    return true;
  }

  if (normalizeComparable(preferredName) !== normalizeComparable(rawAppName)) {
    return true;
  }

  if (isCompactRawToken(rawAppName) && rawTitle && rawTitle !== rawAppName) {
    return true;
  }

  return false;
}
