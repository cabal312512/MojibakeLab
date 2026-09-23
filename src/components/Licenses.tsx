import { useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { X } from 'lucide-react';
import type { Translate } from '../lib/i18n';

export function Licenses({ t }: { t: Translate }) {
  const dialog = useRef<HTMLDialogElement>(null);
  const [text, setText] = useState('');
  async function open() {
    dialog.current?.showModal();
    if (!text) {
      try {
        setText(await invoke<string>('license_text'));
      } catch (error) {
        setText(String(error));
      }
    }
  }
  return (
    <>
      <button className="license-link" onClick={() => void open()} title={t('licenses')}>
        MIT
      </button>
      <dialog ref={dialog} className="license-dialog" aria-labelledby="license-title">
        <div className="license-heading">
          <strong id="license-title">{t('licenses')}</strong>
          <button
            className="icon-button"
            aria-label={t('close')}
            onClick={() => dialog.current?.close()}
          >
            <X size={16} />
          </button>
        </div>
        <pre tabIndex={0}>{text || '…'}</pre>
      </dialog>
    </>
  );
}
