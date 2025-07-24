/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @format
 */

import React, { useState, useEffect, useRef, useCallback } from 'react';
import useBaseUrl from '@docusaurus/useBaseUrl';
import * as docusaurusTheme from '@docusaurus/theme-common';
import MonacoEditorButton, {
    BUTTON_HEIGHT,
    runOnClickForAtLeastOneSecond,
} from './MonacoEditorButton';
import RunPythonButton from './RunPythonButton';
import { PyodideStatus } from './PyodideStatus';
import Editor from '@monaco-editor/react';
import * as LZString from 'lz-string';
import * as stylex from '@stylexjs/stylex';
import SandboxResults from './SandboxResults';
import {
    monaco,
    setAutoCompleteFunction,
    setGetDefFunction,
    setHoverFunctionForMonaco,
    setInlayHintFunctionForMonaco,
} from './configured-monaco';
import type { editor } from 'monaco-editor';
import type { PyreflyErrorMessage } from './SandboxResults';
import { DEFAULT_SANDBOX_PROGRAM } from './DefaultSandboxProgram';
import { usePythonWorker } from './usePythonWorker';

// Import type for Pyrefly State
export interface PyreflyState {
    updateSource: (source: string) => void;
    getErrors: () => ReadonlyArray<PyreflyErrorMessage>;
    autoComplete: (line: number, column: number) => any;
    gotoDefinition: (line: number, column: number) => any;
    queryType: (line: number, column: number) => any;
    inlayHint: () => any;
}

// Lazy initialization function that will only be called when needed
export async function initializePyreflyWasm(): Promise<any> {
    const pyreflyWasmUninitializedPromise =
        typeof window !== 'undefined'
            ? import('../sandbox/pyrefly_wasm')
            : new Promise<any>((_resolve) => {});

    try {
        const mod = await pyreflyWasmUninitializedPromise;
        await mod.default();
        return await mod;
    } catch (e) {
        console.error(e);
        throw e;
    }
}

// This will be used in the component
let pyreflyWasmInitializedPromise: Promise<any> | null = null;

interface SandboxProps {
    sampleFilename: string;
    isCodeSnippet?: boolean;
    codeSample?: string;
    isInViewport?: boolean;
}

export default function Sandbox({
    sampleFilename,
    isCodeSnippet = false,
    codeSample = DEFAULT_SANDBOX_PROGRAM,
    isInViewport = true,
}: SandboxProps): React.ReactElement {
    const editorRef = useRef<editor.IStandaloneCodeEditor | null>(null);
    const [errors, setErrors] =
        useState<ReadonlyArray<PyreflyErrorMessage> | null>([]);
    const [internalError, setInternalError] = useState('');
    const [loading, setLoading] = useState(true);
    const [pyreService, setPyreService] = useState<PyreflyState | null>(null);
    const [editorHeightforCodeSnippet, setEditorHeightforCodeSnippet] =
        useState<number | null>(null);
    const [model, setModel] = useState<editor.ITextModel | null>(null);
    const [pyodideStatus, setPyodideStatus] = useState<PyodideStatus>(
        PyodideStatus.NOT_INITIALIZED
    );
    const [pythonOutput, setPythonOutput] = useState<string>('');
    const [activeTab, setActiveTab] = useState<string>('errors');
    const [isHovered, setIsHovered] = useState(false);

    // Initialize WebAssembly only when the component is in the viewport
    useEffect(() => {
        // Skip initialization if not in viewport
        if (!isInViewport) {
            return;
        }

        setLoading(true);
        // Initialize the WebAssembly module only when the component is mounted
        if (!pyreflyWasmInitializedPromise) {
            pyreflyWasmInitializedPromise = initializePyreflyWasm();
        }

        pyreflyWasmInitializedPromise
            .then((pyrefly) => {
                setPyreService(new pyrefly.State());
                setLoading(false);
                setInternalError('');
            })
            .catch((e) => {
                setLoading(false);
                setInternalError(JSON.stringify(e));
            });
    }, [isInViewport]); // Re-run when isInViewport changes

    // Need to add createModel handler in case monaco model was not created at mount time
    monaco.editor.onDidCreateModel((_newModel) => {
        const curModel = fetchCurMonacoModelAndTriggerUpdate(sampleFilename);
        setModel(curModel);
        forceRecheck();
    });

    // Recheck when pyre service or model changes
    useEffect(() => {
        forceRecheck();
    }, [pyreService, model]);

    function forceRecheck() {
        if (model == null || pyreService == null) return;

        setAutoCompleteFunction(model, (l: number, c: number) =>
            pyreService.autoComplete(l, c)
        );
        setGetDefFunction(model, (l: number, c: number) =>
            pyreService.gotoDefinition(l, c)
        );
        setHoverFunctionForMonaco(model, (l: number, c: number) =>
            pyreService.queryType(l, c)
        );
        setInlayHintFunctionForMonaco(model, () => pyreService.inlayHint());

        // typecheck on edit
        try {
            const value = model.getValue();
            pyreService.updateSource(value);
            const errors =
                pyreService.getErrors() as ReadonlyArray<PyreflyErrorMessage>;
            monaco.editor.setModelMarkers(
                model,
                'default',
                mapPyreflyErrorsToMarkerData(errors)
            );
            setInternalError('');
            setErrors(errors);
        } catch (e) {
            console.error(e);
            setInternalError(JSON.stringify(e));
            setErrors([]);
        }
    }

    const { runPython } = usePythonWorker({
        setPythonOutput,
        setPyodideStatus,
    });

    // Create a function to run Python code that can be passed a model
    const runPythonCodeFunction = createRunPythonCodeFunction(
        setActiveTab,
        runPython
    );
    const runPythonCodeCallback = useCallback(async () => {
        if (!model) return;

        // Set status to running before executing Python code
        setPyodideStatus(PyodideStatus.RUNNING);
        await runPythonCodeFunction(model);
    }, [model, setActiveTab, setPyodideStatus]);

    function onEditorMount(editor: editor.IStandaloneCodeEditor) {
        const model = fetchCurMonacoModelAndTriggerUpdate(sampleFilename);
        setModel(model);

        if (isCodeSnippet) {
            // Add extra space for buttons on mobile devices
            const extraSpace = isMobile() ? BUTTON_HEIGHT + 5 : 0; // Add 5px as buffer on top of button height
            setEditorHeightforCodeSnippet(
                Math.max(50, editor.getContentHeight() + extraSpace)
            );
        }
        editorRef.current = editor;

        // Add keyboard shortcut for Command+Enter (Mac) or Ctrl+Enter (Windows/Linux)
        editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.Enter, () => {
            if (
                !isCodeSnippet &&
                pyodideStatus !== PyodideStatus.INITIALIZING &&
                pyodideStatus !== PyodideStatus.RUNNING
            ) {
                // Use the model from the editor directly
                const editorModel = editor.getModel();
                if (editorModel) {
                    runOnClickForAtLeastOneSecond(
                        (running) =>
                            setPyodideStatus(
                                running
                                    ? PyodideStatus.RUNNING
                                    : PyodideStatus.FINISHED_RUNNING
                            ),
                        async () => {
                            await runPythonCodeFunction(editorModel);
                        }
                    );
                }
            }
        });
    }

    const handleGoToDefFromErrors = (
        startLineNumber: number,
        startColumn: number,
        endLineNumber: number,
        endColumn: number
    ) => {
        const editor = editorRef.current;
        if (editor === null) {
            return;
        }
        const range = {
            startLineNumber,
            startColumn,
            endLineNumber,
            endColumn,
        };

        editor.revealRange(range);
        editor.setSelection(range);
    };

    const buttons = getMonacoButtons(
        isCodeSnippet,
        model,
        runPythonCodeCallback,
        pyodideStatus,
        setPyodideStatus,
        forceRecheck,
        codeSample
    );
    return (
        <div
            id="sandbox-editor"
            {...stylex.props(
                styles.tryEditor,
                !isCodeSnippet && !isMobile() && styles.sandboxPadding
            )}
        >
            <div
                id="sandbox-code-editor-container"
                {...stylex.props(
                    styles.codeEditorContainer,
                    isCodeSnippet && styles.codeEditorContainerWithRadius
                )}
                onMouseEnter={() => setIsHovered(true)}
                onMouseLeave={() => setIsHovered(false)}
            >
                {getPyreflyEditor(
                    isCodeSnippet,
                    sampleFilename,
                    codeSample,
                    forceRecheck,
                    onEditorMount,
                    editorHeightforCodeSnippet
                )}
                {
                    <div
                        {...stylex.props(
                            styles.buttonContainerBase,
                            isMobile()
                                ? isCodeSnippet
                                    ? styles.mobileButtonContainerCodeSnippet
                                    : styles.mobileButtonContainerSandbox
                                : styles.desktopButtonContainer,
                            // show button if it's in sandbox or if it's hovered or if it's mobile
                            // We only want to hide this if it's a code snippet not hovered on mobile
                            !isCodeSnippet || isHovered || isMobile()
                                ? styles.visibleButtonContainer
                                : styles.hiddenButtonContainer
                        )}
                    >
                        {buttons}
                    </div>
                }
            </div>
            {!isCodeSnippet && (
                <SandboxResults
                    loading={loading}
                    goToDef={handleGoToDefFromErrors}
                    errors={errors}
                    internalError={internalError}
                    pythonOutput={pythonOutput}
                    pyodideStatus={pyodideStatus}
                    activeTab={activeTab}
                    setActiveTab={setActiveTab}
                />
            )}
        </div>
    );
}

function updateURL(code: string): void {
    const compressed = LZString.compressToEncodedURIComponent(code);
    const newURL = `${window.location.pathname}?code=${compressed}`;
    window.history.replaceState({}, '', newURL);
}

function getCodeFromURL(): string | null {
    if (typeof window === 'undefined') return null;
    const params = new URLSearchParams(window.location.search);
    const code = params.get('code');
    return code ? LZString.decompressFromEncodedURIComponent(code) : null;
}

function fetchCurMonacoModelAndTriggerUpdate(
    fileName: string
): editor.ITextModel | null {
    const model = monaco.editor
        .getModels()
        .filter((model) => model?.uri?.path === `/${fileName}`)[0];

    if (model == null) {
        return null;
    }

    const codeFromUrl = getCodeFromURL();
    if (codeFromUrl != null && model != null) {
        model.setValue(codeFromUrl);
    }

    // Force update to trigger initial inlay hint
    model.setValue(model.getValue());

    // Ensure tab size is correctly set
    model.updateOptions({ tabSize: 4, insertSpaces: true });

    return model;
}

function isMobile(): boolean {
    return /iPhone|iPad|iPod|Android/i.test(navigator.userAgent);
}

function getPyreflyEditor(
    isCodeSnippet: boolean,
    fileName: string,
    codeSample: string,
    forceRecheck: () => void,
    onEditorMount: (editor: editor.IStandaloneCodeEditor) => void,
    editorHeightforCodeSnippet: number | null
): React.ReactElement {
    const { colorMode } = docusaurusTheme.useColorMode();

    const editorTheme = colorMode === 'dark' ? 'vs-dark' : 'vs-light';
    if (isCodeSnippet) {
        return (
            <Editor
                defaultPath={fileName}
                defaultValue={codeSample}
                defaultLanguage="python"
                theme={editorTheme}
                onChange={forceRecheck}
                onMount={onEditorMount}
                keepCurrentModel={true}
                height={editorHeightforCodeSnippet}
                options={{
                    readOnly: isMobile(),
                    domReadOnly: isMobile(),
                    minimap: { enabled: false },
                    hover: { enabled: true, above: false },
                    scrollBeyondLastLine: false,
                    overviewRulerBorder: false,
                }}
            />
        );
    } else {
        // TODO (T217559369): Instead of manually calculating the sandbox height, we should
        // use flexbox behavior to make the sandbox height to be 75% of the screen
        // This doesn't seem to work with the monaco editor currently.
        const screenHeight = window.innerHeight;
        const navbarElement = document.querySelector(
            '.navbar'
        ) as HTMLElement | null; // Replace with your navbar selector
        const navbarHeight = navbarElement?.offsetHeight || 0;

        const sandboxHeight = ((screenHeight - navbarHeight) * 75) / 100;

        return (
            <Editor
                defaultPath={fileName}
                defaultValue={codeSample}
                defaultLanguage="python"
                theme={editorTheme}
                onChange={(value) => {
                    forceRecheck();
                    if (typeof value === 'string') {
                        updateURL(value);
                    }
                }}
                onMount={onEditorMount}
                keepCurrentModel={true}
                height={sandboxHeight}
                options={{
                    minimap: { enabled: false },
                    hover: { enabled: true, above: false },
                    scrollBeyondLastLine: false,
                    overviewRulerBorder: false,
                }}
            />
        );
    }
}

function getMonacoButtons(
    isCodeSnippet: boolean,
    model: editor.ITextModel,
    runPythonCodeCallback: () => Promise<void>,
    pyodideStatus: PyodideStatus,
    setPyodideStatus: React.Dispatch<React.SetStateAction<PyodideStatus>>,
    forceRecheck: () => void,
    codeSample: string
): ReadonlyArray<React.ReactElement> {
    let buttons: ReadonlyArray<React.ReactElement> = [];
    if (isCodeSnippet) {
        buttons = [
            <OpenSandboxButton model={model} />,
            getCopyButton(model),
            /* Hide reset button if it's readonly, which is when it's a code snippet on mobile */
            !isMobile()
                ? getResetButton(model, forceRecheck, codeSample, isCodeSnippet)
                : null,
        ].filter(Boolean);
    } else {
        buttons = [
            getRunPythonButton(
                runPythonCodeCallback,
                pyodideStatus,
                setPyodideStatus
            ),
            getShareUrlButton(),
            getResetButton(model, forceRecheck, codeSample, isCodeSnippet),
            getGitHubIssuesButton(),
        ];
    }

    // react requires a unique key for each element in an array
    // Apply sandboxMobileButton style to buttons when they're in the sandbox (not code snippet) on mobile
    return buttons.map((button, index) =>
        React.cloneElement(button, {
            key: `button-${index}`,
            // Apply additional style for sandbox buttons on mobile
            ...(!isCodeSnippet && isMobile()
                ? { className: stylex.props(styles.sandboxMobileButton) }
                : {}),
        })
    );
}

// Monaco Editor Buttons
function getShareUrlButton(): React.ReactElement {
    return (
        <MonacoEditorButton
            id="share-url-button"
            onClick={() => {
                const currentURL = window.location.href;
                return navigator.clipboard.writeText(currentURL);
            }}
            defaultLabel="📋 Share URL"
            runningLabel="✓ URL Copied!" // we reuse the running label to indicate that the URL has been copied
            ariaLabel="share URL button"
        />
    );
}

function getGitHubIssuesButton(): React.ReactElement {
    return (
        <MonacoEditorButton
            id="github-issues-button"
            onClick={() => {
                window.open(
                    'https://github.com/facebook/pyrefly/issues/new/choose',
                    '_blank',
                    'noopener,noreferrer'
                );
                return Promise.resolve();
            }}
            defaultLabel="⚠️ Report Issue"
            runningLabel="⚠️ Report Issue"
            ariaLabel="report issue on GitHub"
        />
    );
}

function OpenSandboxButton({
    model,
}: {
    model: editor.ITextModel;
}): React.ReactElement {
    // This call is a react hook that must be called inside the function body rather than the return statement
    const sandboxBaseUrl = useBaseUrl('sandbox/');

    return (
        <MonacoEditorButton
            id="open-sandbox-button"
            onClick={async () => {
                if (model) {
                    const currentCode = model.getValue();
                    const compressed =
                        LZString.compressToEncodedURIComponent(currentCode);
                    // Navigate to the sandbox URL with the compressed code as a query parameter
                    const sandboxURL = sandboxBaseUrl + `?code=${compressed}`;
                    window.location.href = sandboxURL;
                }

                return Promise.resolve();
            }}
            defaultLabel="🧪 Open in Sandbox"
            runningLabel="⏳ Opening Sandbox..."
            ariaLabel="open in sandbox"
        />
    );
}

function getRunPythonButton(
    runPython: () => Promise<void>,
    pyodideStatus: PyodideStatus,
    setPyodideStatus: React.Dispatch<React.SetStateAction<PyodideStatus>>
): React.ReactElement {
    return (
        <RunPythonButton
            runPython={runPython}
            pyodideStatus={pyodideStatus}
            setPyodideStatus={setPyodideStatus}
        />
    );
}

function getCopyButton(model: editor.ITextModel): React.ReactElement {
    return (
        <MonacoEditorButton
            id="copy-code-button"
            onClick={async () => {
                if (model) {
                    const currentCode = model.getValue();
                    await navigator.clipboard.writeText(currentCode);
                }
                return Promise.resolve();
            }}
            defaultLabel="📋 Copy"
            runningLabel="✓ Copied!"
            ariaLabel="copy code to clipboard"
        />
    );
}

function getResetButton(
    model: editor.ITextModel | null,
    forceRecheck: () => void,
    codeSample: string,
    isCodeSnippet: boolean
): React.ReactElement {
    return (
        <MonacoEditorButton
            id="reset-button"
            onClick={async () => {
                if (model) {
                    model.setValue(codeSample);
                    if (!isCodeSnippet) {
                        updateURL(codeSample);
                    }
                    forceRecheck();
                }
            }}
            defaultLabel="🔄 Reset"
            runningLabel="✓ Reset!"
            ariaLabel="reset to default code"
        />
    );
}

// Reusable function to create a Python code runner
export function createRunPythonCodeFunction(
    onActiveTabChange: React.Dispatch<React.SetStateAction<string>>,
    runPython: (code: string) => Promise<void>
) {
    // Return a function that takes a model parameter
    return async (model: editor.ITextModel | null) => {
        if (!model) return;

        // Switch to output tab
        onActiveTabChange('output');

        const code = model.getValue();
        await runPython(code);
    };
}

/**
 * Maps PyreflyErrorMessage array to Monaco editor IMarkerData array
 */
function mapPyreflyErrorsToMarkerData(
    errors: ReadonlyArray<PyreflyErrorMessage>
): editor.IMarkerData[] {
    return errors.map((error) => {
        const message = error.message_details
            ? `${error.message_header}\n${error.message_details}`
            : error.message_header;
        return {
            startLineNumber: error.startLineNumber,
            startColumn: error.startColumn,
            endLineNumber: error.endLineNumber,
            endColumn: error.endColumn,
            message: message,
            severity: error.severity,
        };
    });
}

// Styles for Sandbox component
const styles = stylex.create({
    tryEditor: {
        display: 'flex',
        flexDirection: 'column',
        flex: 1,
    },
    sandboxPadding: {
        paddingHorizontal: '10px',
    },
    codeEditorContainer: {
        position: 'relative',
        display: 'flex',
        overflow: 'visible',
        borderBottom: '10px',
        background: 'var(--color-background)',
        height: '100%',
    },
    codeEditorContainerWithRadius: {
        border: '1px solid var(--color-background-secondary)',
        borderRadius: '0.25rem',
    },
    buttonContainerBase: {
        position: 'absolute',
        display: 'flex',
        zIndex: 10, // used to ensure it's beneath the navbar
        gap: '8px',
    },
    mobileButtonContainerCodeSnippet: {
        // Position at bottom right for mobile
        bottom: '16px',
        right: '16px',
    },
    mobileButtonContainerSandbox: {
        // Position at bottom right for mobile
        bottom: '16px',
        right: '16px',
        flexDirection: 'column', // Buttons stack vertically on mobile for sandbox
    },
    // Style for desktop buttons (hidden by default, visible on hover)
    desktopButtonContainer: {
        // Position at top right for desktop
        top: '20px',
        right: '20px',
    },
    // Style for hidden button Container
    hiddenButtonContainer: {
        opacity: 0,
        visibility: 'hidden',
        transition: 'opacity 0.2s ease-in-out, visibility 0.2s ease-in-out',
    },
    // Style for visible button Container
    visibleButtonContainer: {
        // Visible when hovered
        opacity: 1,
        visibility: 'visible',
        transition: 'opacity 0.2s ease-in-out, visibility 0.2s ease-in-out',
    },
    // Style for sandbox mobile buttons
    sandboxMobileButton: {
        '@media (max-width: 768px)': {
            margin: '0', // No margin needed as gap is handled by the container
            width: '100%', // Make buttons full width on mobile for sandbox
        },
    },
});
