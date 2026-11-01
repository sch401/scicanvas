const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { test } = require('node:test');
const vm = require('node:vm');

function page() {
  const elements = new Map();
  const document = { querySelector(selector) {
    if (!elements.has(selector)) elements.set(selector, {
      innerHTML: '', textContent: '', hidden: false,
      querySelectorAll: () => [], addEventListener() {},
    });
    return elements.get(selector);
  }};
  const html = fs.readFileSync(path.join(__dirname, 'parity.html'), 'utf8');
  const context = vm.createContext({ document, window: {} });
  vm.runInContext(html.match(/<script>([\s\S]*?)<\/script>/)[1], context);
  return { elements, api: vm.runInContext('({BASE, state, overrides, subItems, renderRows, normalizeStatus})', context) };
}

test('all stable IDs render once and each module has one header', () => {
  const { api, elements } = page();
  assert.equal(api.BASE.length, 514);
  assert.equal(new Set(api.BASE.map(r => r.id)).size, 514);
  assert.deepEqual(JSON.parse(JSON.stringify(api.BASE)), JSON.parse(fs.readFileSync(path.join(__dirname, 'parity.json'), 'utf8')));
  const html = elements.get('#rows').innerHTML;
  assert.equal((html.match(/class="id"/g) || []).length, 514);
  assert.equal((html.match(/class="group"/g) || []).length, new Set(api.BASE.map(r => r.module)).size);
});

test('sub-item counts keep accepted, pending acceptance and remaining work distinct', () => {
  const { api } = page();
  const row = api.BASE.find(r => r.id === 'F201');
  const html = api.subItems(row);
  assert.match(html, /已完成 0 \/ 4，待验收 2，部分实现 0，未开始 2/);
  assert.match(html, /class="r" style="width:50.0%"/);
  assert.equal(api.subItems(api.BASE[0]), '');
});

test('search includes sub-item names and notes, feature overrides keep audited counts', () => {
  const { api, elements } = page();
  api.state.q = '显函数参考目录';
  api.renderRows();
  assert.equal(elements.get('#count').textContent, '显示 1 / 514');
  assert.match(elements.get('#rows').innerHTML, /F514/);
  api.overrides.set('F514', '已完成');
  api.state.status.add('已完成');
  api.renderRows();
  assert.match(elements.get('#rows').innerHTML, /已完成 0 \/ 239/);
  api.state.q = '移动平均、Savitzky';
  api.state.status.clear();
  api.renderRows();
  assert.match(elements.get('#rows').innerHTML, /F201/);
});

test('sub-item labels and scope notes escape markup and quotes', () => {
  const { api } = page();
  const html = api.subItems({ sub_items: { '<script>"': { total: 2, done: 1, partial: 1, note: '<img src=x onerror=alert(1)>' } } });
  assert.ok(!html.includes('<script>'));
  assert.ok(!html.includes('<img'));
  assert.match(html, /&lt;script&gt;&quot;/);
  assert.match(html, /未开始 0/);
});

test('legacy prototype overrides use current audited status', () => {
  const { api } = page();
  assert.equal(api.normalizeStatus('F014', '原型'), '部分实现');
  assert.equal(api.normalizeStatus('F105', '原型'), '已实现待验收');
  assert.equal(api.normalizeStatus('F001', '__proto__'), null);
});


// 使用生产端同一个校验器，保证页面嵌入字段与生成规则一致，避免复制第二份 schema。
const { spawnSync } = require('node:child_process');
const evidenceFixture = {
  ...JSON.parse(fs.readFileSync(path.join(__dirname, 'parity.json'), 'utf8'))[0],
  needs: ['auto', 'reference', 'visual', 'manual'],
  evidence: {
    auto: ['crates/scicanvas-core/src/csv.rs::units_from_header'],
    reference: ['samples/README.md'], visual: ['samples/README.md'],
    manual: { checklist: 'docs/acceptance/worksheet.md#F027', result: '通过', date: '2026-11-01', build: 'test-fixture' },
  },
};
function validateEvidence(row) {
  return spawnSync(process.env.PYTHON || 'python', ['-c',
    "import sys,json;sys.path.insert(0,'docs/parity');from build_parity import validate;validate([json.load(sys.stdin)])"],
    { cwd: path.join(__dirname, '../..'), input: JSON.stringify(row), encoding: 'utf8' }).status;
}
const invalidEvidence = [
  r => r.needs = ['bad'], r => r.needs = ['auto', 'auto'],
  r => r.evidence.other = [],
  r => r.evidence.auto = ['missing.rs::test'],
  r => r.evidence.auto = ['crates/scicanvas-core/src/csv.rs::missing_function'],
  r => r.evidence.reference = ['missing.json'], r => r.evidence.visual = ['missing.md'],
  r => r.evidence.manual.checklist = 'missing.md#F027',
  r => r.evidence.manual.checklist = 'docs/acceptance/worksheet.md#F999',
  r => r.evidence.manual.result = '完成', r => r.evidence.manual.build = '',
  r => r.evidence.auto = [], r => r.evidence.reference = [], r => r.evidence.visual = [],
  r => r.evidence.manual.result = '待做',
];
invalidEvidence.forEach((change, rule) => test(`evidence rule ${rule}: valid accepted row and rejected mutation`, () => {
  assert.equal(validateEvidence(evidenceFixture), 0);
  const row = JSON.parse(JSON.stringify(evidenceFixture));
  change(row);
  assert.notEqual(validateEvidence(row), 0);
}));
test('optional evidence and pending manual results remain readable in the generated page', () => {
  const { api } = page();
  const row = api.BASE.find(r => r.id === 'F027');
  assert.ok(row.needs.includes('manual'));
  assert.equal(row.evidence.manual.result, '待做');
  const optional = { ...evidenceFixture };
  delete optional.needs; delete optional.evidence;
  assert.equal(validateEvidence(optional), 0);
  for (const result of ['待做', '未通过', '通过']) {
    const pending = JSON.parse(JSON.stringify(evidenceFixture));
    pending.status = '已实现待验收'; pending.evidence.manual.result = result;
    assert.equal(validateEvidence(pending), 0);
  }
});
