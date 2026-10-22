"""Who started a bench job: recorded at `session start`, `a2test run` and `a2film`, from
the environment the command runs in (testbench/README.md, "Who started it").

What Claude Code puts into the environment of every shell command it runs, read out of
the CLI (2.1.x) and checked here:

    CLAUDECODE=1                  any command run by Claude Code
    CLAUDE_CODE_SESSION_ID        the session; a subagent's commands carry its MAIN
                                  session's id (subagents run inside that session)
    AI_AGENT=claude-code_<v>_agent   only on the commands of a subagent (a main session's
                                  commands carry no AI_AGENT, or the process's own
                                  `..._harness`); the subagent's own id is not in it
    CLAUDE_CODE_SESSION_ATTENDED  1 when a person is attending the session
    CLAUDE_JOB_DIR                a background session's job directory (state.json
                                  holds the session's name)

So the session and main-or-subagent are reliable. Which subagent is not in the
environment; it is known only when the subagent works in a worktree of its own, which
Claude Code names `.claude/worktrees/agent-<id>`. The parent is the main session: its
transcript is `~/.claude/projects/<cwd with every non-alphanumeric as ->/<session>.jsonl`,
so its working directory is found by matching that name against the repository's
worktrees. Without CLAUDECODE the command was started by hand (or by a script a person
ran): `by=human`, with the user name.
"""
import getpass
import json
import os
import re
import subprocess
import time
from pathlib import Path

from . import config


def _git(path, *args):
    try:
        r = subprocess.run(['git', '-C', str(path), *args], capture_output=True, text=True, timeout=10)
    except (OSError, subprocess.TimeoutExpired):
        return ''
    return r.stdout.strip() if r.returncode == 0 else ''


def checkout(path):
    """A checkout's path, worktree name, branch and commit (`-dirty` with uncommitted changes)."""
    p = Path(path).resolve()
    top = _git(p, 'rev-parse', '--show-toplevel') or str(p)
    return dict(path=top, worktree=Path(top).name, branch=_git(top, 'rev-parse', '--abbrev-ref', 'HEAD'),
                commit=_git(top, 'describe', '--always', '--dirty', '--abbrev=10'))


def _slug(path):
    return re.sub(r'[^A-Za-z0-9]', '-', str(path))


def worktrees(repo):
    """[(path, branch)] of every worktree of the repository `repo` belongs to, the main one first."""
    out, cur = [], None
    for line in _git(repo, 'worktree', 'list', '--porcelain').splitlines():
        if line.startswith('worktree '):
            cur = [line[9:], '']
            out.append(cur)
        elif line.startswith('branch ') and cur:
            cur[1] = line[7:].replace('refs/heads/', '')
    return [tuple(x) for x in out]


def session_cwd(session, projects=None):
    """The main session's working directory, as the slug its transcript is filed under."""
    projects = Path(projects or Path.home() / '.claude' / 'projects')
    try:
        for d in projects.iterdir():
            if (d / f'{session}.jsonl').exists():
                return d.name
    except OSError:
        pass
    return None


def identity(env=None, cwd=None, repo=None, projects=None):
    env = os.environ if env is None else env
    cwd = Path(cwd or os.getcwd())
    rec = dict(recorded=round(time.time(), 1))
    rec.update(checkout(repo or config.REPO))         # the checkout the bench runs from
    here = _git(cwd, 'rev-parse', '--show-toplevel')
    if here and Path(here) != Path(rec['path']):
        rec['cwd'] = here                             # the caller works elsewhere
    sid = env.get('CLAUDE_CODE_SESSION_ID')
    if env.get('CLAUDECODE') != '1' and not sid:
        try:
            user = getpass.getuser()
        except Exception:
            user = env.get('USER', '?')
        rec.update(by='human', user=user)
        return rec
    rec.update(by='claude', session=sid or '',
               role='subagent' if env.get('AI_AGENT', '').endswith('_agent') else 'main',
               attended=env.get('CLAUDE_CODE_SESSION_ATTENDED') == '1')
    m = re.match(r'claude-code_([\d-]+)_', env.get('AI_AGENT', ''))
    if m:
        rec['version'] = m.group(1).replace('-', '.')
    if rec['role'] == 'subagent':
        m = re.search(r'/\.claude/worktrees/(agent-[0-9a-f]+)(?:/|$)', str(here or cwd) + '/')
        if m:
            rec['agent'] = m.group(1)
    slug = session_cwd(sid, projects) if sid else None
    if slug:
        for path, branch in worktrees(rec['path']):
            if _slug(path) == slug:
                rec['parent'] = dict(path=path, worktree=Path(path).name, branch=branch)
                break
        else:
            rec['parent'] = dict(slug=slug)
    jd = env.get('CLAUDE_JOB_DIR')
    if jd:
        try:
            name = json.loads((Path(jd) / 'state.json').read_text()).get('name')
            if name:
                rec['name'] = name
        except (OSError, ValueError):
            pass
    return rec


def short(rec):
    """The few words the dashboard shows: `Claude Code d44142c5`, `subagent of bench-tools`,
    `by hand · cedric`."""
    if not rec:
        return 'not recorded'
    if rec.get('by') == 'human':
        return f'by hand · {rec.get("user", "?")}'
    sid = (rec.get('session') or '')[:8]
    if rec.get('role') == 'subagent':
        p = rec.get('parent') or {}
        return f'subagent of {p.get("worktree") or ("session " + sid)}'
    return f'Claude Code {sid}'.strip()


def full(rec):
    """One sentence with everything recorded, for a tooltip."""
    if not rec:
        return 'Who started this was not recorded (a checkout from before testbench 3.42).'
    where = (f'in the checkout {rec.get("worktree")} (branch {rec.get("branch") or "?"}) at '
             f'{rec.get("commit") or "?"}')
    if rec.get('by') == 'human':
        return f'Started by hand ({rec.get("user")}; no Claude Code session in the environment), {where}.'
    p = rec.get('parent') or {}
    pw = p.get('worktree') or p.get('slug') or 'an unknown directory'
    s = rec.get('session') or '?'
    if rec.get('role') == 'subagent':
        who = (f'a subagent{" (" + rec["agent"] + ")" if rec.get("agent") else ""} of Claude Code session '
               f'{s}, whose main session works in {pw}')
    else:
        who = f'Claude Code, main session {s}{" (" + rec["name"] + ")" if rec.get("name") else ""}, working in {pw}'
    return (f'Started by {who}{"; attended" if rec.get("attended") else "; unattended"}, {where}.')
