function normalizeSummary(text) {
  return (text || '').replace(/\s+/g, ' ').trim();
}

function splitSummarySentences(text) {
  return normalizeSummary(text)
    .split(/[。！？!?]/)
    .map((item) => item.trim())
    .filter(Boolean);
}

function splitSummaryClauses(text) {
  return text
    .split(/[，,；;、]/)
    .map((item) => item.trim())
    .filter(Boolean);
}

function ensureChineseStop(text) {
  if (!text) {
    return '';
  }
  return `${text.replace(/[。！？!?]+$/g, '').trim()}。`;
}

export function getFullSummary(text) {
  return normalizeSummary(text);
}

export function getPrimarySummary(text) {
  const normalized = normalizeSummary(text);
  if (!normalized) {
    return '';
  }

  const firstSentence = splitSummarySentences(normalized)[0] || normalized;
  const clauses = splitSummaryClauses(firstSentence);

  if (clauses.length >= 3) {
    return clauses.slice(0, 2).join('，');
  }

  return firstSentence;
}

export function getSecondarySummary(text) {
  const normalized = normalizeSummary(text);
  if (!normalized) {
    return '';
  }

  const sentences = splitSummarySentences(normalized);
  const firstSentence = sentences[0] || normalized;
  const clauses = splitSummaryClauses(firstSentence);

  if (clauses.length >= 3) {
    return ensureChineseStop(clauses.slice(2).join('，'));
  }

  if (sentences.length > 1) {
    return ensureChineseStop(sentences.slice(1).join('。'));
  }

  return '';
}

export function getMainApps(mainApps) {
  return (mainApps || '')
    .split(/[，,]/)
    .map((item) => item.trim())
    .filter(Boolean)
    .slice(0, 4);
}

export function getSummaryRhythmMeta(totalDuration = 0) {
  if (totalDuration >= 45 * 60) {
    return { tone: 'deep', label: '深度推进' };
  }

  if (totalDuration >= 20 * 60) {
    return { tone: 'steady', label: '持续推进' };
  }

  return { tone: 'light', label: '轻量切换' };
}
