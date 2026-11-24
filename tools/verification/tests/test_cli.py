"""Behavioral contracts for focused selection and completion verification."""
from concurrent.futures import ThreadPoolExecutor
import contextlib
from hashlib import sha256
import io
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch
from git_fixtures import restore_repository
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from cli import parse_args
from commands import select_phases
import impact
import verify
import commands as selection
import execution
from timing_report import REPORT_NAME
import timing_report
from impact import RustScope

def selected(sentence, changes=()):
    _, args = parse_args(sentence.split() if isinstance(sentence, str) else sentence)
    return [item for _, commands in select_phases(args, set(changes)) for item in commands]
from test_case_report import ignore_archive

class SelectionTests(unittest.TestCase):

    def test_startup_help_shows_nested_command_tree_without_selecting_work(self):
        for argv in ([], ['--help']):
            with self.subTest(argv=argv), contextlib.redirect_stdout(io.StringIO()) as output:
                if argv:
                    with self.assertRaises(SystemExit) as exit_result:
                        parse_args(argv)
                    self.assertEqual(exit_result.exception.code, 0)
                else:
                    _, args = parse_args(argv)
                    self.assertIsNone(args)
            help_text = output.getvalue()
            for command in ('milestone [singlestep]', 'crate NAME [suite NAME] [case FILTER]', 'python [directory PATH [file PATTERN] [case FILTER]]', 'singlestep [case FILTER] [full|sample N]', 'architecture [case FILTER]', 'english [files PATH... | removals PATH]', 'analyze RUN_DIRECTORY',
                            'check unused dependencies\n', 'check unused dependencies crate cpu\n',
                            'check unused public crate cpu\n'):
                self.assertIn(command, help_text)

    def test_focused_rust_preserves_suite_and_case_without_completion(self):
        commands = selected('test rust crate test_runner suite test_singlestep case test_sub')
        self.assertEqual(len(commands), 1)
        self.assertEqual(commands[0][1], [sys.executable, 'tools/verification/run_singlestep.py', '--profile', 'test', '--case', 'test_sub'])
        self.assertEqual(commands[0][2], {})

    def test_exact_selection_targets_one_named_rust_test_or_singlestep_group(self):
        rust = selected('test rust crate cpu suite test_add case test_add_b --exact')
        self.assertEqual(rust[0][1][-2:], ['test_add_b', '--exact'])
        silicon = selected('test runner singlestep case test_add_b sample 5 --exact')
        self.assertEqual(silicon[0][1][-3:], ['--case', 'test_add_b', '--exact'])
        self.assertEqual(silicon[0][2], {'SINGLESTEP_FULL': '0', 'SINGLESTEP_LIMIT': '5'})

    def test_exact_without_supported_case_is_rejected(self):
        for sentence in ('test rust unit --exact', 'test rust crate cpu --exact', 'test rust crate cpu case test_add_b --exact', 'test python directory tools/verification/tests case test_cli --exact', 'test runner standard --exact', 'test runner singlestep --exact', 'commit --exact'):
            with self.subTest(sentence=sentence), self.assertRaises(ValueError):
                selected(sentence)

    def test_runner_build_compiles_test_and_benchmark_targets_without_execution(self):
        commands = selected('runner build')
        self.assertEqual(len(commands), 1)
        self.assertEqual(commands[0][1], ['cargo', 'test', '--release', '--no-run', '-p', 'test_runner', '--tests', '--benches'])

    def test_python_selection_preserves_directory_file_and_case(self):
        suites = (
            ('Bootstrap contracts', 'tools/bootstrap/tests'),
            ('Graphify contracts', 'tools/graphify/tests'),
            ('Conversion contracts', 'tools/bootstrap/conversion/tests'),
            ('HTML conversion contracts', 'tools/bootstrap/html-to-markdown/tests'),
            ('PDF conversion contracts', 'tools/bootstrap/pdf-to-markdown/tests'),
            ('Book review contracts', 'tools/bootstrap/book-to-markdown/tests'),
            ('RAG MCP contracts', 'tools/amiga-rag-mcp-server/tests'),
            ('Artifact report contracts', 'tools/artifacts/tests'),
            ('Verification CLI contracts', 'tools/verification/tests'),
            ('Workflow capture contracts', 'tools/workflow/capture/tests'),
        )
        repo = Path(__file__).resolve().parents[3]
        owning_changes = [f'{directory}/test_trial.py' for _, directory in suites]
        batch = selected('test python', reversed(owning_changes))
        self.assertEqual([name for name, _, _ in batch], [name for name, _ in suites])
        for (name, directory), automatic in zip(suites, batch, strict=True):
            for target in (directory, str(repo / directory)):
                with self.subTest(name=name, target=target):
                    if name == 'Book review contracts':
                        runner = [sys.executable, 'tools/bootstrap/book-to-markdown/run_tests.py']
                    else:
                        runner = [sys.executable, 'tools/verification/run_tests.py', '-s', target, '-q']
                    focused, = selected(['test', 'python', 'directory', target,
                                         'file', 'test_*.py', 'case', 'trial'])
                    self.assertEqual(focused, ('Python contracts', [*runner, '-p', 'test_*.py', '-k', 'trial'], {}))
                    if target == directory:
                        self.assertEqual(automatic, (name, runner, {}))

        defaults = {name: command for name, command, _ in selected('test python', ['tools/verification/run_tests.py'])}
        self.assertEqual(defaults['Verification CLI contracts'], [sys.executable, 'tools/verification/run_tests.py', '-s', 'tools/verification/tests', '-q'])
        self.assertEqual(set(defaults), {'Verification CLI contracts', 'Workflow capture contracts'})
        capture = 'Workflow capture contracts'
        runner = [sys.executable, 'tools/verification/run_tests.py', '-s', 'tools/workflow/capture/tests', '-q']
        self.assertEqual(defaults[capture], runner)
        commit = {name: command for name, command, _ in selected('commit', ['tools/workflow/capture/tests/test_trial.py'])}
        self.assertEqual(commit[capture], runner)

    def test_focused_python_keeps_unregistered_directories_on_standard_discovery(self):
        with tempfile.TemporaryDirectory(prefix='unregistered python suite ') as directory:
            command, = selected(['test', 'python', 'directory', directory,
                                 'file', 'test_sample.py', 'case', 'test_sample'])
            self.assertEqual(command, ('Python contracts', [sys.executable, 'tools/verification/run_unittest.py',
                              'discover', '-s', directory, '-q', '-p', 'test_sample.py', '-k', 'test_sample'], {}))

    def test_default_python_selection_skips_unrelated_changes(self):
        for changes in ([], ['crates/cpu/src/state.rs'], ['docs/testing.md'], ['tools/bootstrap/conversion-old/client.py']):
            self.assertEqual(selected('test python', changes), [])

    def test_default_python_selection_covers_owned_code_and_shared_dependencies(self):
        parallel_suites = {'Verification CLI contracts', 'Workflow capture contracts'}
        cases = (('tools/bootstrap/ensure_qdrant.ps1', {'Bootstrap contracts'}), ('tools/bootstrap/bootstrap_graphify.ps1', {'Bootstrap contracts'}), ('tools/graphify/refresh.py', {'Graphify contracts'}), ('tools/graphify/semantic.py', {'Graphify contracts'}), ('tools/graphify/read.py', {'Graphify contracts'}), ('tools/graphify/tests/test_corpora.py', {'Graphify contracts'}), ('tools/bootstrap/tests/test_bootstrap_contract.py', {'Bootstrap contracts'}), ('tools/bootstrap/html-to-markdown/pipeline.py', {'Conversion contracts', 'HTML conversion contracts'}), ('tools/bootstrap/conversion/client.py', {'Conversion contracts', 'HTML conversion contracts', 'PDF conversion contracts', 'Book review contracts'}), ('tools/bootstrap/pdf-to-markdown/pipeline.py', {'Conversion contracts', 'PDF conversion contracts', 'Book review contracts'}), ('tools/bootstrap/book-to-markdown/review/model.py', {'Book review contracts'}), ('tools/amiga-rag-mcp-server/server.py', {'RAG MCP contracts'}), ('.codex/config.toml', {'RAG MCP contracts'}), ('tools/artifacts/report.py', {'Artifact report contracts'}), ('tools/artifacts/tests/test_report.py', {'Artifact report contracts'}), ('tools/workflow/capture/records.py', {'Workflow capture contracts'}), ('tools/workflow/capture/tests/test_records.py', {'Workflow capture contracts'}), ('tools/verification/tests/git_fixtures.py', parallel_suites), ('tools/verification/case_report.py', parallel_suites), ('tools/verification/repository.py', parallel_suites), ('tools/verification/commands.py', {'Verification CLI contracts'}), ('tools/web.ps1', {'Verification CLI contracts'}))
        for path, expected in cases:
            with self.subTest(path=path):
                self.assertEqual({item[0] for item in selected('test python', [path])}, expected)

    def test_capture_contracts_follow_owning_changes_and_keep_full_rust_fallback(self):
        group = 'Workflow capture contracts'
        for path in ('tools/workflow/capture/records.py', 'tools/workflow/capture/tests/test_records.py'):
            with self.subTest(path=path):
                names = {item[0] for item in selected('commit', [path])}
                self.assertTrue({group, 'All ordinary crate tests', 'All standard harness tests', 'RAG MCP contracts', 'Artifact report contracts'} <= names)
        for changes in ([], ['DIARY.md'], ['tools/workflow/result.py'], ['crates/cpu/src/cpu.rs']):
            with self.subTest(changes=changes):
                self.assertNotIn(group, {item[0] for item in selected('commit', changes)})

    def test_root_ignore_rules_select_both_consumers_and_keep_full_rust_scope(self):
        changes = ['.gitignore']
        consumers = {'Verification CLI contracts', 'Workflow capture contracts'}
        self.assertEqual({name for name, _, _ in selected('test python', changes)}, consumers)
        commit = {name for name, _, _ in selected('commit', changes)}
        self.assertTrue(consumers | {'All ordinary crate tests', 'All standard harness tests'} <= commit)

    def test_full_and_sampled_silicon_runs_override_inherited_selection(self):
        commands = selected('test runner singlestep case test_add_b sample 500')
        self.assertEqual(commands[0][2], {'SINGLESTEP_FULL': '0', 'SINGLESTEP_LIMIT': '500'})
        self.assertEqual(commands[0][1][-1], 'test_add_b')
        self.assertEqual(selected('test runner singlestep')[0][2], {'SINGLESTEP_FULL': '1', 'SINGLESTEP_LIMIT': 'all'})

    def test_commit_excludes_full_singlestep_even_for_cpu_changes(self):
        for changes in ([], ['crates/cpu/src/cpu.rs'], ['Cargo.toml'], ['crates/cpu/tests/test_cck_bus.rs'], ['docs/testing.md']):
            commands = selected('commit', changes)
            self.assertFalse(any(('tools/verification/run_singlestep.py' in item[1] for item in commands)))

    def test_commit_checks_obsidian_frontmatter_without_git_change_selection(self):
        for changes in ([], ['tools/verification/commands.py']):
            commands = selected('commit', changes)
            self.assertTrue(any((item[1][-1] == 'tools/verification/checks/check_obsidian_frontmatter.py' for item in commands)))

    def test_commit_always_builds_the_browser_application(self):
        web = selected('check web')
        self.assertEqual(len(web), 1)
        self.assertEqual(web[0][0], 'WebAssembly build')
        self.assertEqual(web[0][1][-4:], ['-NoProfile', '-File', 'tools/web.ps1', 'build'])
        for changes in ([], ['docs/testing.md'], ['crates/gui/src/app.rs'], ['Cargo.toml']):
            with self.subTest(changes=changes):
                commands = selected('commit', changes)
                self.assertEqual([item for item in commands if item[0] == 'WebAssembly build'], web)
        self.assertIn(web[0], selected('check all'))

    def test_empty_standard_harness_never_selects_all_cargo_targets(self):
        with patch.object(Path, 'glob', return_value=[Path('test_singlestep.rs')]):
            commands = selected('commit')
            self.assertFalse(any((item[1][:2] == ['cargo', 'test'] for item in commands)))
            self.assertEqual(selected('test runner standard'), [])

    def test_standard_harness_keeps_explicit_ordinary_targets(self):
        with patch.object(Path, 'glob', return_value=[Path('test_singlestep.rs'), Path('test_reports.rs')]):
            commands = selection.harness_commands(['standard'])
            self.assertEqual(commands[0][1], ['cargo', 'test', '--no-fail-fast', '-p', 'test_runner', '--test', 'test_reports', '--', '--quiet'])

    def test_explicit_singlestep_runs_once_alongside_automatic_selection(self):
        commands = selected('milestone singlestep', ['crates/cpu/src/cpu.rs'])
        self.assertEqual(sum(('tools/verification/run_singlestep.py' in item[1] for item in commands)), 1)
        self.assertEqual(selected('test runner silicon')[0][1:], selected('test runner singlestep')[0][1:])

    def test_ordinary_cpu_wait_tests_remain_in_normal_selection(self):
        changes = ['crates/cpu/tests/test_cck_bus.rs']
        unit, = selected('test rust unit', changes)
        self.assertEqual(unit[0], 'Tier 1 ordinary crate tests')
        self.assertEqual(unit[1][:2], ['cargo', 'test'])
        self.assertEqual(unit[1][unit[1].index('cpu') - 1], '-p')
        commands = {name: command for name, command, _ in selected('commit', changes)}
        ordinary = commands['Ordinary crate tests']
        self.assertEqual([value for option, value in zip(ordinary, ordinary[1:]) if option == '-p'], ['cpu', 'debugger', 'gui', 'machine_loop', 'save_state'])
        self.assertNotIn('--workspace', ordinary)
        self.assertIn('All standard harness tests', commands)

    def test_milestone_retains_separate_scope(self):
        for changes in ([], ['crates/cpu/src/cpu.rs'], ['docs/testing.md']):
            commands = selected('milestone', changes)
            self.assertEqual(commands, [('Full SingleStepTests', [sys.executable, 'tools/verification/run_singlestep.py', '--profile', 'verification'], {'SINGLESTEP_FULL': '1', 'SINGLESTEP_LIMIT': 'all'})])

    def test_commit_skips_rust_but_keeps_quality_and_python_suites_for_bootstrap_changes(self):
        commands = selected('commit', ['tools/bootstrap/book-to-markdown/review/service.py'])
        self.assertFalse(any(('llvm-cov' in item[1] or item[1][:2] == ['cargo', 'test'] for item in commands)))
        self.assertTrue(any((any((part.endswith('checks/check_architecture.py') for part in item[1])) for item in commands)))
        suites = (
            ('Bootstrap contracts', 'tools/bootstrap/tests'),
            ('Graphify contracts', 'tools/graphify/tests'),
            ('Conversion contracts', 'tools/bootstrap/conversion/tests'),
            ('HTML conversion contracts', 'tools/bootstrap/html-to-markdown/tests'),
            ('PDF conversion contracts', 'tools/bootstrap/pdf-to-markdown/tests'),
            ('RAG MCP contracts', 'tools/amiga-rag-mcp-server/tests'),
            ('Artifact report contracts', 'tools/artifacts/tests'),
        )
        runner = 'tools/verification/run_tests.py'
        self.assertEqual(
            [item for item in commands if runner in item[1] or 'discover' in item[1]],
            [(name, [sys.executable, runner, '-s', directory, '-q'], {}) for name, directory in suites],
        )
        self.assertTrue(any(('tools/bootstrap/book-to-markdown/run_tests.py' in item[1] for item in commands)))

    def test_language_data_keeps_commit_gates_without_expanding_rust(self):
        self.enterContext(patch.object(impact, 'workspace_graph', return_value=self.gui_graph()))
        gui_path = 'crates/gui/src/lib.rs'
        docs = selected('commit', ['docs/testing.md'])
        gui = selected('commit', [gui_path])
        names = {name for name, _, _ in docs}
        self.assertTrue({'Architecture Rules', 'Clippy Invariants'} <= names)
        self.assertNotIn('Strict English Policy', names)
        self.assertFalse(any(('llvm-cov' in command or command[:2] == ['cargo', 'test'] for _, command, _ in docs)))
        self.assertNotIn('Verification CLI contracts', names)
        for path, expected in (('tools/verification/checks/english_words/programming.txt', []), ('tools/verification/checks/check_english.json', ['Verification CLI contracts'])):
            with self.subTest(path=path):
                for changes, baseline in (([path], docs), ([gui_path, path], gui)):
                    commands = selected('commit', changes)
                    self.assertEqual(commands[:len(baseline)], baseline)
                    self.assertEqual([name for name, _, _ in commands[len(baseline):]], expected)

    def gui_graph(self):
        return ({selection.REPO_ROOT / 'crates/gui': 'gui'}, {'gui': set()})

    def test_partial_coverage_runs_once_and_exports_the_same_packages(self):
        with patch.object(selection, 'select_rust_scope', return_value=RustScope(('cpu', 'gui', 'test_runner'), 'fixture')):
            commands = selected('commit', ['crates/cpu/src/cpu.rs'])
        run = next((command for _, command, _ in commands if '--no-report' in command))
        self.assertEqual(run[:3], ['cargo', 'llvm-cov', 'nextest'])
        self.assertIn('--no-fail-fast', run)
        for option, value in (('--profile', 'default'), ('--retries', '0'), ('--no-tests', 'fail'), ('--config-file', '.config/nextest.toml')):
            self.assertEqual(run[run.index(option) + 1], value)
        export = next((command for _, command, _ in commands if '--json' in command))
        self.assertNotIn('--summary-only', export)
        self.assertEqual(export[-2:], ['--output-path', '.agent/tmp/coverage/coverage.json'])
        summary = next((command for _, command, _ in commands if 'tools/verification/coverage_report.py' in command and '--prepare' not in command))
        for command in (run, export):
            self.assertEqual([command[i + 1] for i, word in enumerate(command) if word == '-p'], ['cpu', 'gui'])
            self.assertNotIn('--workspace', command)
        self.assertEqual(summary[-4:], ['--package', 'cpu', '--package', 'gui'])
        plain = [command for _, command, _ in commands if command[:2] == ['cargo', 'test']]
        self.assertEqual(len(plain), 1)
        self.assertIn('test_runner', plain[0])
        self.assertNotIn('test_singlestep', plain[0])

    def test_orchestration_keeps_all_quality_gates_and_python_contracts(self):
        self.enterContext(patch.object(impact, 'workspace_graph', return_value=self.gui_graph()))
        gates = {'Obsidian Frontmatter', 'Formatting', 'Clippy Invariants', 'Architecture Rules', 'Hardware Quality', 'Code Quality', 'WebAssembly build', 'Graphify contracts', 'RAG MCP contracts', 'Artifact report contracts'}
        self.assertEqual({name for name, _, _ in selected('commit', ['docs/testing.md'])}, gates)
        gates.add('Verification CLI contracts')
        coverage = {'Prepare coverage report', 'Reset coverage counters', 'Ordinary crate tests', 'Export ordinary crate coverage', 'Ordinary crate coverage summary'}
        for path in ('tools/verification/verify.py', 'tools/verification/execution.py', 'tools/verification/evidence.py', 'tools/verification/batch_lock.py', 'tools/verification/tests/test_evidence.py'):
            with self.subTest(path=path):
                self.assertEqual({name for name, _, _ in selected('commit', [path])}, gates)
                gui = {name: command for name, command, _ in selected('commit', ['crates/gui/src/lib.rs', path])}
                self.assertEqual(set(gui), gates | coverage)
                run = gui['Ordinary crate tests']
                self.assertEqual([value for option, value in zip(run, run[1:]) if option == '-p'], ['gui'])
                self.assertNotIn('--workspace', run)
        for path in ('tools/verification/commands.py', 'tools/verification/impact.py', 'tools/verification/repository.py', 'tools/verification/new_module.py'):
            with self.subTest(path=path):
                commands = selected('commit', ['tools/verification/evidence.py', path])
                run = next((command for _, command, _ in commands if '--no-report' in command))
                self.assertIn('--workspace', run)
                self.assertIn('All standard harness tests', {name for name, _, _ in commands})

    def test_development_coverage_reuses_pipeline_without_completion_gates(self):
        for sentence, package in (('test coverage crate cpu', 'cpu'), ('test coverage workspace', None)):
            with self.subTest(sentence=sentence):
                commands = selected(sentence)
                self.assertEqual(len(commands), 5)
                run = next((command for _, command, _ in commands if '--no-report' in command))
                self.assertNotIn('cargo test', ' '.join(run))
                if package:
                    self.assertEqual(run[run.index('-p') + 1], package)
                    self.assertNotIn('--workspace', run)
                else:
                    self.assertIn('--workspace', run)
                    self.assertEqual(run[run.index('--exclude') + 1], 'test_runner')
                self.assertFalse(any(('check_' in ' '.join(command) for _, command, _ in commands)))
                self.assertFalse(any(('run_singlestep.py' in ' '.join(command) for _, command, _ in commands)))

    def test_coverage_rejects_unknown_packages_harness_and_partial_test_filters(self):
        for sentence in ('test coverage crate unknown_package', 'test coverage crate test_runner', 'test coverage crate cpu suite test_add', 'test coverage workspace --exact'):
            with self.subTest(sentence=sentence), self.assertRaises(ValueError):
                selected(sentence)

    def test_unaffected_runner_is_skipped_and_harness_only_change_has_no_coverage(self):
        for packages, harness, coverage in ((('gui',), False, True), (('test_runner',), True, False), ((), False, False)):
            with self.subTest(packages=packages), patch.object(selection, 'select_rust_scope', return_value=RustScope(packages, 'fixture')):
                commands = selected('commit', ['docs/testing.md'])
            self.assertEqual(any((command[:2] == ['cargo', 'test'] for _, command, _ in commands)), harness)
            self.assertEqual(any(('llvm-cov' in command for _, command, _ in commands)), coverage)
            self.assertIn([sys.executable, 'tools/verification/checks/check_clippy_policy.py'], [command for _, command, _ in commands])

    def test_commit_skips_bootstrap_contracts_without_bootstrap_changes(self):
        bootstrap = {'Bootstrap contracts', 'Conversion contracts', 'HTML conversion contracts', 'PDF conversion contracts', 'Book review contracts'}
        for changes in ([], ['crates/cpu/src/cpu.rs'], ['tools/verification/commands.py']):
            with self.subTest(changes=changes):
                names = {name for name, _, _ in selected('commit', changes)}
                self.assertFalse(names & bootstrap)
                self.assertIn('RAG MCP contracts', names)
                self.assertIn('Artifact report contracts', names)
                self.assertEqual('Verification CLI contracts' in names, any((selection.verification_contract_input(path) for path in changes)))
        for path in ('tools/bootstrap/conversion/client.py', 'tools/bootstrap/books/new/source.json'):
            with self.subTest(path=path):
                names = {name for name, _, _ in selected('commit', [path])}
                self.assertEqual(names & bootstrap, bootstrap)

    def test_focused_architecture_filter_does_not_select_other_checks(self):
        commands = selected('check architecture case test_file_size_limits')
        self.assertEqual(len(commands), 1)
        self.assertEqual(commands[0][1][:-1], [sys.executable, 'tools/verification/checks/check_architecture.py'])
        self.assertEqual(commands[0][1][-1], 'test_file_size_limits')

    def test_explicit_language_and_all_keep_the_scanner(self):
        scanner = [sys.executable, 'tools/verification/checks/check_english.py']
        plain = ('Strict English Policy', scanner, {})
        self.assertEqual(selected('check english'), [plain])
        self.assertEqual(selected('check english files docs/testing.md AGENTS.md'), [('Strict English Policy', scanner + ['--', 'docs/testing.md', 'AGENTS.md'], {})])
        self.assertEqual(selected(['check', 'english', 'files', '--', '--removals']),
                         [('Strict English Policy', scanner + ['--', '--removals'], {})])
        proposal = '.agent/tmp/removal proposal.json'
        self.assertEqual(selected(['check', 'english', 'removals', proposal]),
                         [('Strict English Policy', scanner + ['--removals', proposal], {})])
        all_checks = selected('check all')
        self.assertEqual([name for name, _, _ in all_checks], ['Strict English Policy', 'Obsidian Frontmatter', 'Formatting', 'Clippy Invariants', 'WebAssembly build', 'Architecture Rules', 'Hardware Quality', 'Code Quality'])
        self.assertEqual([command for _, command, _ in all_checks if scanner[1] in command], [scanner])

    def test_settings_can_follow_subcommands_without_eating_runner_arguments(self):
        _, args = parse_args(['runner', 'bench', '--log-level', 'full', '--plan', 'case', 'add/add_w_d1_d0'])
        self.assertTrue(args.plan)
        self.assertEqual(args.log_level, 'full')
        command = select_phases(args, set())[0][1][0][1]
        self.assertEqual(command[1:3], ['tools/verification/cpu_performance_tests.py', 'measure'])
        self.assertEqual(command[-2:], ['case', 'add/add_w_d1_d0'])

    def test_report_output_is_visible_by_default_and_can_be_overridden(self):
        self.assertEqual(parse_args(['runner', 'analyze', 'fixture.csv'])[1].log_level, 'full')
        self.assertEqual(parse_args(['runner', 'analyze', 'fixture.csv', '--log-level', 'error'])[1].log_level, 'error')
        self.assertEqual(parse_args(['commit'])[1].log_level, 'warn')

    def test_benchmark_selects_cpu_performance_tests(self):
        self.assertEqual(selected('runner bench case add/add_w_d1_d0')[0][1][1:], ['tools/verification/cpu_performance_tests.py', 'measure', 'case', 'add/add_w_d1_d0'])
        self.assertEqual(selected('runner analyze full_run')[0][1][1:], ['tools/verification/cpu_performance_tests.py', 'analyze', 'full_run'])
        self.assertEqual(selected('runner bench cases selected.txt')[0][1][1:], ['tools/verification/cpu_performance_tests.py', 'measure', 'cases', 'selected.txt'])
        self.assertEqual(selected('runner refresh filtered_run')[0][1][1:], ['tools/verification/cpu_performance_tests.py', 'refresh', 'filtered_run'])

    def test_refresh_allows_intended_current_snapshot_change(self):
        with tempfile.TemporaryDirectory() as temporary, patch.object(verify, 'REPO_ROOT', Path(temporary)), patch.object(verify, 'execute_command', return_value=(0, 'refreshed', 0.1)), contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(verify.main(['runner', 'refresh', 'filtered_run']), 0)

    def test_gui_keeps_its_separate_native_options(self):
        self.assertEqual(selected('runner gui -- --frames 10')[0][1][-2:], ['--frames', '10'])

    def test_invalid_sentences_are_rejected(self):
        for sentence in ('test rust suite missing', 'test rust crate cpu crate gui', 'test python directory missing', 'test runner dma', 'test runner harness', 'test runner dma sample 50', 'milestone dma', 'test runner singlestep sample 0', 'test runner singlestep sample 50 full', 'milestone singlestep singlestep', 'check english case name', 'runner bench -- --quick', 'runner bench measure retry', 'runner bench duration-ms 0', 'runner bench series 0', 'runner bench case add/add_w_d1_d0 mnemonic ADD', 'runner bench measure filter ADD', 'runner bench measure unroll 500', 'runner bench measure passes 9', 'runner bench measure output reports', 'runner bench measure pin off', 'runner bench unroll'):
            with self.subTest(sentence=sentence), self.assertRaises(ValueError):
                selected(sentence)

    def test_removals_requires_exactly_one_proposal_path(self):
        for extra in ([], [' '], ['proposal.json', 'other.json']):
            with self.subTest(extra=extra), self.assertRaisesRegex(ValueError, 'removals PATH'):
                selected(['check', 'english', 'removals', *extra])

class CommandExecutionTests(unittest.TestCase):

    def test_exact_rust_run_rejects_zero_matching_tests(self):
        for count, expected in ((0, 1), (1, 0)):
            process = unittest.mock.MagicMock()
            process.__enter__.return_value = process
            process.stdout = [f"running {count} test{('s' if count != 1 else '')}\n"]
            process.wait.return_value = 0
            with self.subTest(count=count), tempfile.TemporaryDirectory() as temporary, patch.object(execution.subprocess, 'Popen', return_value=process), contextlib.redirect_stdout(io.StringIO()):
                code, _, _ = execution.run_cmd(['cargo', 'test', '--', 'case', '--exact'], log_dir=temporary)
            self.assertEqual(code, expected)

class ExecutionTests(unittest.TestCase):

    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        restore_repository('empty', self.root)
        ignore_archive(self.root)
        self.enterContext(patch.object(verify, 'REPO_ROOT', self.root))

    def english_scanner_path(self):
        scanner = self.root / 'tools/verification/checks/check_english.py'
        scanner.parent.mkdir(parents=True, exist_ok=True)
        return scanner

    def test_unused_dispatch_and_previews_use_only_the_selected_command(self):
        results = self.root / 'reported'
        forms = (
            ([], ['cargo', 'check', '--workspace']),
            (['crate', 'cpu'], ['cargo', 'check', '-p', 'cpu']),
            (None, [sys.executable, 'tools/verification/unused_public.py', 'cpu', '--results-dir',
                    str(results)]),
        )
        for package, command in forms:
            words = ['public', 'crate', 'cpu'] if package is None else ['dependencies', *package]
            arguments = ['check', 'unused', *words, '--results-dir', str(results)]
            with self.subTest(words=words), \
                 patch.object(selection, 'workspace_packages', return_value=([], {self.root: 'cpu'})), \
                 patch.object(verify, 'execute_command', return_value=(0, 'ok', 0)) as execute, \
                 contextlib.redirect_stdout(io.StringIO()) as output:
                self.assertEqual(verify.main([*arguments, '--plan']), 0)
                execute.assert_not_called()
                self.assertFalse(results.exists())
                self.assertIn(subprocess.list2cmdline(command), output.getvalue())
                self.assertEqual(verify.main(arguments), 0)
                execute.assert_called_once()
                self.assertEqual(execute.call_args.args[0], command)
                self.assertEqual(execute.call_args.kwargs.get('check_unused', False), package is not None)
                self.assertEqual(execute.call_args.kwargs['log_dir'], results / 'verify')

    def test_invalid_unused_selectors_stop_before_preview_or_dispatch(self):
        invalid = [[], ['stray'], ['public']]
        for mode in ('dependencies', 'public'):
            invalid.extend([mode, *words] for words in (
                ['crate'], ['crate', ''], ['crate', 'missing'], ['crate', 'cpu@0.1.0'],
                ['crate', 'cpu', 'crate', 'cpu'], ['crate', 'cpu', 'other'],
                ['crate', 'cpu', 'file', 'crates/cpu/src/cpu.rs'], ['crate', 'cpu', 'line', '12'],
            ))
        for words in invalid:
            unknown = bool(words and words[-1] in ('missing', 'cpu@0.1.0'))
            for options in ([], ['--plan']):
                with self.subTest(words=words, options=options), \
                     patch.object(selection, 'workspace_packages', return_value=([], {self.root: 'cpu'})) as packages, \
                     patch.object(verify, 'execute_command') as execute, \
                     contextlib.redirect_stderr(io.StringIO()) as diagnostics, \
                     self.assertRaises(SystemExit) as status:
                    verify.main(['check', 'unused', *words, *options])
                self.assertEqual(status.exception.code, 2)
                execute.assert_not_called()
                self.assertEqual(packages.call_count, int(unknown))
                expected = 'Unknown workspace package' if unknown else 'Use check unused dependencies'
                self.assertIn(expected, diagnostics.getvalue())

    def test_english_requires_a_readable_report_from_its_child(self):
        scanner = self.english_scanner_path()
        path = self.root / '.agent/tmp/english/example.json'
        path.parent.mkdir(parents=True)
        marker = '[INFO] English scan report: .agent/tmp/english/example.json | scope=repository'
        valid = {'scope': 'repository', 'requested_files': [], 'target_count': 0,
                 'scanned_count': 0, 'complete': True, 'exit_code': 0, 'findings': []}
        for label, content, markers, args in (
            ('unannounced', json.dumps(valid), [], []),
            ('missing', None, [marker], []),
            ('malformed', '{', [marker], []),
            ('wrong status', json.dumps({**valid, 'exit_code': 1}), [marker], []),
            ('incomplete success', json.dumps({**valid, 'complete': False}), [marker], []),
            ('missing finding fields', json.dumps({**valid, 'findings': [{'kind': 'unused_entry_candidate'}]}), [marker], []),
            ('multiple', json.dumps(valid), [marker, marker], []),
            ('failed removal with success status', json.dumps({**valid, 'validation': {'accepted': False}}),
             [marker], ['removals', 'proposal.json']),
        ):
            with self.subTest(label=label):
                if content is None:
                    path.unlink(missing_ok=True)
                else:
                    path.write_text(content, encoding='utf-8')
                scanner.write_text(''.join(f'print({line!r})\n' for line in markers), encoding='utf-8')
                with patch.object(execution, 'REPO_ROOT', self.root), contextlib.redirect_stdout(io.StringIO()) as output:
                    self.assertEqual(verify.main(['check', 'english', *args]), 2, output.getvalue())
                self.assertIn('Cannot use English scan report:', output.getvalue())
                self.assertIn('(scanner exit code 0)', output.getvalue())
                self.assertNotIn(marker, output.getvalue())

    def test_check_all_keeps_aggregate_failure_and_runs_other_checks(self):
        scanner = self.english_scanner_path()
        report = self.root / '.agent/tmp/english/example.json'
        report.parent.mkdir(parents=True)
        report.write_text(json.dumps({'scope': 'repository', 'requested_files': [], 'target_count': 0,
                                      'scanned_count': 0, 'complete': False, 'exit_code': 2,
                                      'findings': [{'kind': 'scan_error', 'phase': 'configuration',
                                                    'diagnostic': 'Malformed configuration'}]}), encoding='utf-8')
        marker = '[INFO] English scan report: .agent/tmp/english/example.json | scope=repository'
        scanner.write_text(f'print({marker!r})\nraise SystemExit(2)\n', encoding='utf-8')
        next_command = ('Next quality check', [sys.executable, '-c', 'print("[INFO] Other check ran.")'], {})
        with patch.object(execution, 'REPO_ROOT', self.root), \
             patch.object(selection, 'select_quality_commands', return_value=[next_command]), \
             contextlib.redirect_stdout(io.StringIO()) as output:
            self.assertEqual(verify.main(['check', 'all']), 1, output.getvalue())
        self.assertIn('command exited with code 2', output.getvalue())
        self.assertIn(marker, output.getvalue())
        self.assertIn('[INFO] Other check ran.', output.getvalue())
        self.assertIn('[FAIL] check verification', output.getvalue())

    def test_english_cli_publishes_unique_reports_with_native_status(self):
        scanner = self.english_scanner_path()
        scripts = Path(verify.__file__).parent
        for script in scripts.glob('*.py'):
            shutil.copyfile(script, scanner.parents[1] / script.name)
        shutil.copyfile(scripts / 'checks/check_english.py', scanner)
        source = self.root / 'notes with spaces.md'
        source.write_text('These are words.\n', encoding='utf-8')
        config = scanner.with_suffix('.json')
        scope = {'word_lists': [], 'file_words': {source.name: ['words']},
                 'always_excluded_directories': [], 'excluded_directories': ['.agent'],
                 'excluded_directory_names': [], 'excluded_files': [], 'text_extensions': ['.md'],
                 'text_filenames': []}
        config.write_text(json.dumps(scope), encoding='utf-8')
        baseline_config = config.read_bytes()
        proposal = self.root / '.agent/tmp/removal proposal.json'
        proposal.parent.mkdir(parents=True)
        proposal.write_text(json.dumps([{'type': 'file_word', 'path': config.relative_to(self.root).as_posix(),
                                        'owner': source.name, 'position': 0, 'spelling': 'words', 'term': 'words'}]),
                            encoding='utf-8')
        environment = {**os.environ, 'AMIGA_TEST_TASK_ID': 'english-cli-fixture',
                       'AMIGA_TEST_EXECUTION_ID': 'english-cli-fixture', 'AMIGA_TEST_PARENT_INVOCATION_ID': ''}
        command = [sys.executable, 'tools/verification/verify.py', 'check', 'english']

        def run(*args):
            return subprocess.run([*command, *args], cwd=self.root, env=environment,
                                  capture_output=True, encoding='utf-8', timeout=60)

        def read_report(result, code, kinds, complete=True):
            self.assertEqual(result.returncode, code, result.stdout + result.stderr)
            reports = [line for line in result.stdout.splitlines() if line.startswith('[INFO] English scan report: ')]
            self.assertEqual(len(reports), 1, result.stdout)
            locator = reports[0].removeprefix('[INFO] English scan report: ').split(' | ')[0]
            report = json.loads((self.root / locator).read_text(encoding='utf-8'))
            self.assertEqual(report['exit_code'], code)
            self.assertEqual(report['complete'], complete)
            self.assertCountEqual([finding['kind'] for finding in report['findings']], kinds)
            return locator, report

        with ThreadPoolExecutor(max_workers=2) as workers:
            concurrent = workers.submit(run, 'files', str(source))
            first = run('files', './' + source.name)
            second = concurrent.result()
        first_locator, first_report = read_report(first, 0, ['unused_entry_candidate'])
        second_locator, _ = read_report(second, 0, ['unused_entry_candidate'])
        self.assertNotEqual(first_locator, second_locator)
        self.assertEqual(first_report['requested_files'], [source.name])
        report_logs = (self.root / '.agent/task-archives/english-cli-fixture/english-cli-fixture/test-runs').glob('*/command-1.log')
        self.assertTrue(any(first_locator in log.read_text(encoding='utf-8') for log in report_logs))

        _, validation = read_report(run('removals', str(proposal)), 0, ['unused_entry_candidate'])
        self.assertIs(validation['validation']['accepted'], True)
        self.assertEqual(config.read_bytes(), baseline_config)
        unknown = 'wrong' * 8
        source.write_text(f'These are words. {unknown}.\n', encoding='utf-8')
        _, with_unknown = read_report(run('files', source.name), 1, ['unknown_word', 'unused_entry_candidate'])
        finding, = (item for item in with_unknown['findings'] if item['kind'] == 'unknown_word')
        self.assertEqual((finding['word'], finding['path'], finding['line']), (unknown, source.name, 1))
        unreadable = self.root / 'unreadable.md'
        unreadable.write_bytes(bytes([255]))
        _, incomplete = read_report(run('files', source.name, unreadable.name), 2,
                                    ['unknown_word', 'unused_entry_candidate', 'scan_error'], complete=False)
        self.assertEqual((incomplete['target_count'], incomplete['scanned_count']), (2, 1))
        config.write_text('{', encoding='utf-8')
        read_report(run('files', source.name), 2, ['scan_error'], complete=False)

    def test_cold_profiles_launch_the_remaining_commands_and_keep_gate_failures(self):
        note = self.root / 'docs/testing.md'
        note.parent.mkdir()
        note.write_text('The notes changed.\n', encoding='utf-8')
        quality = ['tools/verification/checks/check_obsidian_frontmatter.py', 'fmt', 'tools/verification/checks/check_architecture.py', 'tools/verification/checks/audit_hardware_quality.py', 'tools/verification/checks/audit_code_quality.py', 'tools/verification/checks/check_clippy_policy.py', '-NoProfile']
        expected = [(['commit'], quality, None), (['milestone'], ['tools/verification/run_singlestep.py'], None), (['milestone', 'singlestep'], ['tools/verification/run_singlestep.py'], None), (['commit'], quality[:5], 'tools/verification/checks/check_architecture.py')]
        native = verify.execute_command
        for index, (arguments, launched, failed) in enumerate(expected):
            with self.subTest(arguments=arguments, fail=failed):
                observed = []

                def substitute(command, overrides, **options):
                    observed.append((command, overrides))
                    child = f'raise SystemExit({int(command[1] == failed)})'
                    if 'SINGLESTEP_RESULTS_DIR' in overrides:
                        child = "import os; from pathlib import Path; out = Path(os.environ['SINGLESTEP_RESULTS_DIR']); out.mkdir(parents=True); [(out / name).write_text('{}') for name in ('run.json', 'timing.json', 'summary.json', 'execution.log')]"
                    return native([sys.executable, '-c', child], overrides, **options)
                with (patch.object(verify, 'execute_command', side_effect=substitute),
                      patch.object(execution, 'REPO_ROOT', self.root),
                      patch.object(subprocess, 'check_output', return_value='rustc 1.99.0') as compiler_probe,
                      patch('urllib.request.urlopen') as release_probe,
                      contextlib.redirect_stdout(io.StringIO()) as output):
                    release_probe.return_value.__enter__.return_value.read.return_value = (
                        b'[pkg.rust]\nversion = "1.100.0"\n')
                    code = verify.main([*arguments, '--results-dir', f'.agent/tmp/profile-test/{index}'])
                self.assertEqual(code, int(failed is not None), output.getvalue())
                release_probe.assert_not_called()
                compiler_probe.assert_not_called()
                self.assertNotIn('Stable update', output.getvalue())
                observed_gates = [(command, overrides) for command, overrides in observed if any((part in quality or part.endswith('/run_singlestep.py') for part in command))]
                self.assertCountEqual([command[1] for command, _ in observed_gates], launched)
                self.assertFalse(any((part.endswith('/check_english.py') for command, _ in observed for part in command)))
                if arguments[0] == 'milestone':
                    (command, overrides), = observed
                    self.assertEqual(command[-2:], ['--profile', 'verification'])
                    self.assertEqual([overrides[name] for name in ('SINGLESTEP_FULL', 'SINGLESTEP_LIMIT')], ['1', 'all'])

    def test_plan_shows_isolated_logs_and_singlestep_reports_without_writing(self):
        with patch.object(verify, 'execute_command') as execute, contextlib.redirect_stdout(io.StringIO()) as output:
            self.assertEqual(verify.main(['test', 'runner', 'singlestep', 'case', 'test_add_b', '--exact', '--results-dir', '.agent/tmp/experiments/add', '--plan']), 0)
        execute.assert_not_called()
        self.assertIn(f"[PLAN] Verifier logs: {self.root / '.agent/tmp/experiments/add/verify'}", output.getvalue())
        self.assertIn('SINGLESTEP_RESULTS_DIR', output.getvalue())
        self.assertIn(repr(str(self.root / '.agent/tmp/experiments/add/singlestep')), output.getvalue())
        self.assertFalse((self.root / '.agent/tmp/verification/diagnostics').exists())

    def test_results_directory_reaches_execution_and_singlestep_environment(self):
        with patch.object(verify, 'execute_command', return_value=(0, 'ok', 0)) as execute, contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(verify.main(['test', 'runner', 'singlestep', 'case', 'test_add_b', '--results-dir', '.agent/tmp/experiments/add']), 0)
        self.assertEqual(execute.call_args.kwargs['log_dir'], self.root / '.agent/tmp/experiments/add/verify')
        self.assertEqual(execute.call_args.args[1]['SINGLESTEP_RESULTS_DIR'], str(self.root / '.agent/tmp/experiments/add/singlestep'))

    def test_diagnostic_reports_are_unique_and_external_logs_keep_their_route(self):
        diagnostics = self.root / '.agent/tmp/verification/diagnostics'
        with tempfile.TemporaryDirectory() as temporary, patch.object(verify, 'execute_command', side_effect=[(0, 'ok', 1), (1, 'failed', 2)]) as execute, contextlib.redirect_stdout(io.StringIO()) as output:
            external = Path(temporary)
            args = ['test', 'runner', 'singlestep', 'case', 'test_add_b', '--results-dir', str(external)]
            self.assertEqual(verify.main(args), 0, output.getvalue())
            first, = diagnostics.glob('*/test-runtime-report.md')
            first_bytes = first.read_bytes()
            self.assertEqual(verify.main(args), 1, output.getvalue())
            second, = set(diagnostics.glob('*/test-runtime-report.md')) - {first}
            self.assertEqual(first.read_bytes(), first_bytes)
            self.assertIn('executed', first_bytes.decode('utf-8'))
            failure = second.read_text(encoding='utf-8')
            self.assertIn('run_singlestep.py --profile verification --case test_add_b | failed | 2.000 |', failure)
            self.assertNotIn('| Stability |', failure)
            self.assertNotIn('Source or index changed', output.getvalue())
            self.assertIn(str(first), output.getvalue())
            self.assertIn(str(second), output.getvalue())
            for call in execute.call_args_list:
                self.assertEqual(call.kwargs['log_dir'], external / 'verify')
                self.assertEqual(call.args[1]['SINGLESTEP_RESULTS_DIR'], str(external / 'singlestep'))
            self.assertEqual(list(external.rglob('test-runtime-report.md')), [])

    def read_diagnostic_report(self, output):
        diagnostics = self.root / '.agent/tmp/verification/diagnostics'
        report, = (path for path in diagnostics.glob(f'*/{REPORT_NAME}') if str(path) in output)
        content = report.read_bytes()
        self.assertIn(f'[TIMING] Report SHA-256: {sha256(content).hexdigest()}', output)
        return content.decode('utf-8')

    def test_diagnostic_interrupt_retains_completed_and_interrupted_commands(self):
        (self.root / '.gitignore').write_text('.agent/tmp/\n', encoding='utf-8')
        with patch.object(verify, 'execute_command', side_effect=[(0, 'ok', 1.25), KeyboardInterrupt()]) as execute, contextlib.redirect_stdout(io.StringIO()) as output:
            self.assertEqual(verify.main(['test', 'python']), 130)
        self.assertEqual(execute.call_count, 2)
        report = self.read_diagnostic_report(output.getvalue())
        self.assertIn('tools/verification/tests -q | executed | 1.250 |', report)
        self.assertIn('tools/workflow/capture/tests -q | interrupted |', report)
        self.assertIn('Verification interrupted', output.getvalue())

    def test_diagnostic_command_error_preserves_completed_measurement(self):
        (self.root / '.gitignore').write_text('.agent/tmp/\n', encoding='utf-8')
        for error in (OSError('command unavailable'), RuntimeError('command unavailable')):
            with self.subTest(error=type(error).__name__), patch.object(verify, 'execute_command', side_effect=[(0, 'ok', 1.25), error]) as execute, contextlib.redirect_stdout(io.StringIO()) as output:
                self.assertEqual(verify.main(['test', 'python']), 1)
            self.assertEqual(execute.call_count, 2)
            report = self.read_diagnostic_report(output.getvalue())
            self.assertIn('tools/verification/tests -q | executed | 1.250 |', report)
            self.assertIn('tools/workflow/capture/tests -q | failed |', report)
            self.assertIn('Diagnostic execution failed: command unavailable', output.getvalue())

    def test_diagnostic_coverage_preparation_failure_retains_pending_scopes(self):
        with patch.object(verify, 'execute_command', return_value=(1, 'preparation failed', 2)) as execute, contextlib.redirect_stdout(io.StringIO()) as output:
            self.assertEqual(verify.main(['test', 'coverage', 'workspace']), 1)
        execute.assert_called_once()
        report = self.read_diagnostic_report(output.getvalue())
        self.assertIn('coverage_report.py --prepare | failed | 2.000 |', report)
        pending = [row.split(' | ')[1:4] for row in report.splitlines() if ' | missing | ' in row]
        self.assertEqual([(group, status) for group, _, status in pending], [('Reset coverage counters', 'missing'), ('All ordinary crate tests', 'missing'), ('Export ordinary crate coverage', 'missing'), ('Ordinary crate coverage summary', 'missing')])
        for (_, scope, _), selector in zip(pending, ('--profraw-only', '--workspace --exclude test_runner', 'cargo llvm-cov report --json', 'coverage_report.py')):
            self.assertIn(selector, scope)

    def test_diagnostic_report_publication_failure_exits_without_receipt(self):
        original_open = Path.open

        def fail_report_open(path, mode='r', *args, **kwargs):
            if path.name == REPORT_NAME and mode == 'xb':
                raise OSError('write unavailable')
            return original_open(path, mode, *args, **kwargs)
        for failure, message in ((patch.object(Path, 'open', fail_report_open), 'write unavailable'), (patch.object(timing_report, 'git', side_effect=RuntimeError('revision unavailable')), 'revision unavailable')):
            with self.subTest(message=message), failure, patch.object(verify, 'execute_command', return_value=(0, 'ok', 4)) as execute, contextlib.redirect_stdout(io.StringIO()) as output:
                self.assertEqual(verify.main(['check', 'english']), 2)
            execute.assert_called_once()
            self.assertIn(f'Cannot write timing report: {message}', output.getvalue())
            self.assertNotIn('[TIMING] Report SHA-256:', output.getvalue())
            self.assertNotIn('[TIMING] Report generation:', output.getvalue())
            diagnostics = self.root / '.agent/tmp/verification/diagnostics'
            self.assertEqual(list(diagnostics.glob(f'*/{REPORT_NAME}')), [])

    def test_inventory_runs_without_candidate_generation(self):
        for success in (True, False):
            with self.subTest(success=success), patch.object(verify, 'execute_command', return_value=(int(not success), 'result', 0)) as execute, contextlib.redirect_stdout(io.StringIO()):
                self.assertEqual(verify.main(['runner', 'bench', 'inventory']), 0 if success else 1)
            self.assertEqual(execute.call_count, 1)
            self.assertEqual(execute.call_args_list[0].args[0][:2], ['cargo', 'bench'])
            self.assertFalse((self.root / '.agent/tmp/verification/diagnostics').exists())

    def test_unrelated_changes_execute_no_python_contracts(self):
        with patch.object(verify, 'collect_changes', return_value={'crates/cpu/src/state.rs'}), patch.object(verify, 'execute_command') as execute:
            with contextlib.redirect_stdout(io.StringIO()) as output:
                self.assertEqual(verify.main(['test', 'python']), 0)
        execute.assert_not_called()
        self.assertIn('inputs are unchanged', output.getvalue())
        self.assertFalse((self.root / '.agent/tmp/verification/diagnostics').exists())

    def test_default_python_command_collects_working_tree_changes(self):
        with patch.object(verify, 'collect_changes', return_value={'tools/amiga-rag-mcp-server/server.py'}) as collect:
            with contextlib.redirect_stdout(io.StringIO()) as output:
                self.assertEqual(verify.main(['test', 'python', '--plan']), 0)
        collect.assert_called_once_with(self.root)
        self.assertIn('RAG MCP contracts', output.getvalue())
        self.assertNotIn('Bootstrap contracts', output.getvalue())

    def test_commit_plan_collects_working_tree_changes(self):
        with patch.object(verify, 'collect_changes', return_value={'tools/bootstrap/conversion/client.py'}) as collect:
            with contextlib.redirect_stdout(io.StringIO()) as output:
                self.assertEqual(verify.main(['commit', '--plan']), 0)
        collect.assert_called_once_with(self.root)
        self.assertIn('Book review contracts', output.getvalue())
        self.assertIn('Rust test scope: none', output.getvalue())
        self.assertIn('previous summary is not evidence', output.getvalue())

    def test_plan_never_executes_commands(self):
        with patch.object(verify, 'execute_command') as execute:
            with contextlib.redirect_stdout(io.StringIO()):
                self.assertEqual(verify.main(['check', 'architecture', '--plan']), 0)
            execute.assert_not_called()
if __name__ == '__main__':
    unittest.main()
