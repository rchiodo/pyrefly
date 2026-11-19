#!/usr/bin/env python3
# Copyright (c) Meta Platforms, Inc. and affiliates.
#
# This source code is licensed under the MIT license found in the
# LICENSE file in the root directory of this source tree.

"""Compare language servers with semantic editor-feature probes."""

import argparse
import json
import os
import queue
import re
import shlex
import subprocess
import tempfile
import threading
from dataclasses import dataclass
from pathlib import Path
from typing import BinaryIO
from urllib.parse import unquote, urlparse

FEATURES = ("quick fixes", "hover", "completion", "navigation", "rename")

LIBRARY = """\
class Greeter:
    def greet(self, name: str) -> str:
        \"\"\"Return a greeting.\"\"\"
        return f"Hello {name}"
"""

QUICK_FIX = """\
Greeter()
"""

HOVER = """\
from library import Greeter

Greeter().greet("World")
"""

COMPLETION = """\
message = "hello"
message.
"""

NAVIGATION = HOVER

RENAME = """\
message = "hello"
print(message)
other_message = message
"""


@dataclass(frozen=True)
class FeatureResult:
    feature: str
    status: str
    evidence: str


def source_position(source: str, needle: str, occurrence: int = 1) -> dict[str, int]:
    start = -1
    for _ in range(occurrence):
        start = source.index(needle, start + 1)
    prefix = source[:start]
    return {
        "line": prefix.count("\n"),
        "character": len(prefix.rsplit("\n", 1)[-1]),
    }


def range_for(source: str, needle: str, occurrence: int = 1) -> dict[str, object]:
    start = source_position(source, needle, occurrence)
    return {
        "start": start,
        "end": {"line": start["line"], "character": start["character"] + len(needle)},
    }


def flatten_strings(value: object) -> str:
    if isinstance(value, str):
        return value
    if isinstance(value, dict):
        return " ".join(flatten_strings(item) for item in value.values())
    if isinstance(value, list):
        return " ".join(flatten_strings(item) for item in value)
    return ""


def completion_labels(response: object) -> set[str]:
    if isinstance(response, dict):
        response = response.get("items", [])
    if not isinstance(response, list):
        return set()
    return {
        item["label"]
        for item in response
        if isinstance(item, dict) and isinstance(item.get("label"), str)
    }


def workspace_edit_texts(edit: object) -> list[str]:
    if not isinstance(edit, dict):
        return []
    texts = []
    changes = edit.get("changes", {})
    if isinstance(changes, dict):
        for edits in changes.values():
            if isinstance(edits, list):
                texts.extend(
                    item["newText"]
                    for item in edits
                    if isinstance(item, dict) and isinstance(item.get("newText"), str)
                )
    document_changes = edit.get("documentChanges", [])
    if isinstance(document_changes, list):
        for change in document_changes:
            if not isinstance(change, dict):
                continue
            edits = change.get("edits", [])
            if isinstance(edits, list):
                texts.extend(
                    item["newText"]
                    for item in edits
                    if isinstance(item, dict) and isinstance(item.get("newText"), str)
                )
    return texts


def code_action_edit_texts(actions: object) -> list[str]:
    if not isinstance(actions, list):
        return []
    return [
        text
        for action in actions
        if isinstance(action, dict)
        for text in workspace_edit_texts(action.get("edit"))
    ]


def location_paths(response: object) -> list[str]:
    if isinstance(response, dict):
        response = [response]
    if not isinstance(response, list):
        return []
    paths = []
    for location in response:
        if not isinstance(location, dict):
            continue
        uri = location.get("uri") or location.get("targetUri")
        if isinstance(uri, str):
            paths.append(unquote(urlparse(uri).path))
    return paths


class JsonRpcClient:
    def __init__(self, command: list[str], cwd: Path) -> None:
        self.process = subprocess.Popen(
            command,
            cwd=cwd,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
        )
        if self.process.stdin is None or self.process.stdout is None:
            raise RuntimeError("language server pipes were not created")
        self.stdin: BinaryIO = self.process.stdin
        self.stdout: BinaryIO = self.process.stdout
        self.messages: queue.Queue[dict[str, object]] = queue.Queue()
        self.next_id = 1
        self.diagnostics: dict[str, list[dict[str, object]]] = {}
        threading.Thread(target=self._read_messages, daemon=True).start()

    def _read_messages(self) -> None:
        while True:
            headers = {}
            while True:
                line = self.stdout.readline()
                if not line:
                    return
                if line == b"\r\n":
                    break
                key, value = line.decode("ascii").split(":", 1)
                headers[key.lower()] = value.strip()
            body = self.stdout.read(int(headers["content-length"]))
            self.messages.put(json.loads(body))

    def send(self, payload: dict[str, object]) -> None:
        body = json.dumps(payload).encode()
        header = f"Content-Length: {len(body)}\r\n\r\n".encode()
        self.stdin.write(header + body)
        self.stdin.flush()

    def notify(self, method: str, params: object) -> None:
        self.send({"jsonrpc": "2.0", "method": method, "params": params})

    def request(self, method: str, params: object) -> object:
        request_id = self.next_id
        self.next_id += 1
        self.send(
            {
                "jsonrpc": "2.0",
                "id": request_id,
                "method": method,
                "params": params,
            }
        )
        while True:
            message = self.messages.get(timeout=20)
            if message.get("id") == request_id and (
                "result" in message or "error" in message
            ):
                if "error" in message:
                    raise RuntimeError(f"{method} failed: {message['error']}")
                return message.get("result")
            self._handle_message(message)

    def _handle_message(self, message: dict[str, object]) -> None:
        method = message.get("method")
        if method == "textDocument/publishDiagnostics":
            params = message.get("params", {})
            if isinstance(params, dict) and isinstance(params.get("uri"), str):
                diagnostics = params.get("diagnostics", [])
                if isinstance(diagnostics, list):
                    self.diagnostics[params["uri"]] = diagnostics
        if "id" not in message or not isinstance(method, str):
            return
        params = message.get("params", {})
        if method == "workspace/configuration":
            items = params.get("items", []) if isinstance(params, dict) else []
            result = [
                {"python": {"analysis": {"autoImportCompletions": True}}}
                for _ in items
            ]
        elif method == "workspace/workspaceFolders":
            result = None
        else:
            result = None
        self.send({"jsonrpc": "2.0", "id": message["id"], "result": result})

    def wait_for_diagnostics(self, uri: str) -> list[dict[str, object]]:
        while uri not in self.diagnostics:
            try:
                self._handle_message(self.messages.get(timeout=5))
            except queue.Empty:
                return []
        return self.diagnostics[uri]

    def close(self) -> None:
        try:
            self.request("shutdown", None)
            self.notify("exit", None)
            self.process.wait(timeout=5)
        except (queue.Empty, subprocess.TimeoutExpired):
            self.process.kill()
            self.process.wait()


def text_document(uri: str, position: dict[str, int]) -> dict[str, object]:
    return {"textDocument": {"uri": uri}, "position": position}


def score_server(command: list[str]) -> tuple[str, list[FeatureResult]]:
    with tempfile.TemporaryDirectory() as temp:
        root = Path(temp)
        sources = {
            "library.py": LIBRARY,
            "quick_fix.py": QUICK_FIX,
            "hover.py": HOVER,
            "completion.py": COMPLETION,
            "navigation.py": NAVIGATION,
            "rename.py": RENAME,
        }
        for name, source in sources.items():
            (root / name).write_text(source)
        uris = {name: (root / name).as_uri() for name in sources}
        client = JsonRpcClient(command, root)
        try:
            initialize = client.request(
                "initialize",
                {
                    "processId": None,
                    "rootUri": root.as_uri(),
                    "capabilities": {
                        "workspace": {"configuration": True},
                        "textDocument": {
                            "codeAction": {},
                            "completion": {},
                            "definition": {},
                            "hover": {"contentFormat": ["markdown", "plaintext"]},
                            "rename": {"prepareSupport": True},
                        },
                    },
                    "workspaceFolders": [
                        {"uri": root.as_uri(), "name": "editor-scorecard"}
                    ],
                },
            )
            version = "unknown"
            if isinstance(initialize, dict):
                server_info = initialize.get("serverInfo", {})
                if isinstance(server_info, dict):
                    version = str(server_info.get("version", "unknown"))
            if version == "unknown":
                pinned = re.search(
                    r"(?:pyright@|pyrefly(?:==|@))([0-9][^ ]*)",
                    " ".join(command),
                )
                if pinned is not None:
                    version = pinned.group(1)
            client.notify("initialized", {})
            for name, source in sources.items():
                client.notify(
                    "textDocument/didOpen",
                    {
                        "textDocument": {
                            "uri": uris[name],
                            "languageId": "python",
                            "version": 1,
                            "text": source,
                        }
                    },
                )

            quick_fix_diagnostics = client.wait_for_diagnostics(uris["quick_fix.py"])
            actions = client.request(
                "textDocument/codeAction",
                {
                    "textDocument": {"uri": uris["quick_fix.py"]},
                    "range": range_for(QUICK_FIX, "Greeter"),
                    "context": {"diagnostics": quick_fix_diagnostics},
                },
            )
            action_edits = code_action_edit_texts(actions)
            quick_fix_ok = any(
                "library" in edit and "Greeter" in edit for edit in action_edits
            )

            hover = client.request(
                "textDocument/hover",
                text_document(
                    uris["hover.py"], source_position(HOVER, "greet", occurrence=1)
                ),
            )
            hover_text = flatten_strings(hover)
            hover_ok = all(
                fragment in hover_text
                for fragment in (
                    "greet",
                    "name: str",
                    "-> str",
                    "Return a greeting.",
                )
            )

            completions = client.request(
                "textDocument/completion",
                text_document(
                    uris["completion.py"],
                    {"line": 1, "character": len("message.")},
                ),
            )
            completion_ok = "upper" in completion_labels(completions)

            definitions = client.request(
                "textDocument/definition",
                text_document(
                    uris["navigation.py"],
                    source_position(NAVIGATION, "greet", occurrence=1),
                ),
            )
            navigation_ok = any(
                Path(path).name == "library.py" for path in location_paths(definitions)
            )

            rename = client.request(
                "textDocument/rename",
                {
                    **text_document(
                        uris["rename.py"], source_position(RENAME, "message")
                    ),
                    "newName": "renamed_message",
                },
            )
            rename_texts = workspace_edit_texts(rename)
            rename_ok = rename_texts.count("renamed_message") == 3

            results = [
                FeatureResult(
                    "quick fixes",
                    "pass" if quick_fix_ok else "fail",
                    "offers an import edit for library.Greeter"
                    if quick_fix_ok
                    else "no import quick fix matched the semantic oracle",
                ),
                FeatureResult(
                    "hover",
                    "pass" if hover_ok else "fail",
                    "includes symbol, typed signature, and docstring"
                    if hover_ok
                    else "missing symbol, signature, or docstring information",
                ),
                FeatureResult(
                    "completion",
                    "pass" if completion_ok else "fail",
                    "includes str.upper for a str receiver"
                    if completion_ok
                    else "str.upper was absent",
                ),
                FeatureResult(
                    "navigation",
                    "pass" if navigation_ok else "fail",
                    "resolves greet to library.py"
                    if navigation_ok
                    else "did not resolve greet to library.py",
                ),
                FeatureResult(
                    "rename",
                    "pass" if rename_ok else "fail",
                    "edits the declaration and two semantic references"
                    if rename_ok
                    else f"expected three edits, received {len(rename_texts)}",
                ),
            ]
            return version, results
        finally:
            client.close()


def render_report(
    checker_results: list[tuple[str, str, list[FeatureResult]]],
) -> str:
    lines = [
        "| Checker | Version | "
        + " | ".join(feature.title() for feature in FEATURES)
        + " | Total |",
        "|---|---|" + "---:|" * (len(FEATURES) + 1),
    ]
    for checker, version, results in checker_results:
        statuses = {result.feature: result.status for result in results}
        total = sum(status == "pass" for status in statuses.values())
        lines.append(
            f"| {checker} | {version} | "
            + " | ".join(statuses[feature] for feature in FEATURES)
            + f" | {total}/{len(FEATURES)} |"
        )
    lines.append("")
    for checker, _, results in checker_results:
        lines.append(f"### {checker} evidence")
        lines.extend(
            f"- **{result.feature.title()}:** {result.status} - {result.evidence}"
            for result in results
        )
        lines.append("")
    return "\n".join(lines).rstrip()


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--checker",
        action="append",
        required=True,
        metavar="NAME=COMMAND",
        help="language server name and shell-style stdio command",
    )
    args = parser.parse_args()
    results = []
    for specification in args.checker:
        name, separator, command = specification.partition("=")
        if not separator:
            parser.error("--checker must use NAME=COMMAND")
        version, scores = score_server(shlex.split(command, posix=os.name != "nt"))
        results.append((name, version, scores))
    print(render_report(results))


if __name__ == "__main__":
    main()
