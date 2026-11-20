export function computeDiff(oldData: any, newData: any): any {
  if (!oldData) return newData;
  const diff: Record<string, any> = {};
  const allKeys = new Set([...Object.keys(oldData), ...Object.keys(newData)]);
  
  for (const key of allKeys) {
    const oldVal = oldData[key];
    const newVal = newData[key];
    if (JSON.stringify(oldVal) !== JSON.stringify(newVal)) {
      diff[key] = { from: oldVal, to: newVal };
    }
  }
  return diff;
}
