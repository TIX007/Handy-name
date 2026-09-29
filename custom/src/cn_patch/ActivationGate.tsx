/**
 * 激活码输入面板组件
 *
 * 展示设备 ID，提供激活码输入框。
 * 激活成功后刷新页面状态。
 *
 * 使用方式（在 App.tsx 中条件渲染）：
 * ```tsx
 * {showActivation && <ActivationGate onActivated={() => refresh()} />}
 * ```
 */

import React, { useState, useEffect, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { commands } from "@/bindings";

interface ActivationGateProps {
  onActivated?: () => void;
}

const ActivationGate: React.FC<ActivationGateProps> = ({ onActivated }) => {
  const { t } = useTranslation();
  const [deviceId, setDeviceId] = useState("");
  const [code, setCode] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fetchDeviceId = useCallback(async () => {
    const result = await commands.cnGetDeviceId();
    if (result.status === "ok") {
      setDeviceId(result.data);
    }
  }, []);

  useEffect(() => {
    fetchDeviceId();
  }, [fetchDeviceId]);

  const handleSubmit = async () => {
    if (!code.trim()) return;
    setLoading(true);
    setError(null);
    try {
      const result = await commands.cnSubmitActivationCode(code.trim());
      if (result.status === "ok") {
        onActivated?.();
      } else {
        setError(result.error);
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="flex flex-col items-center gap-6 p-8 bg-background rounded-xl border border-mid-gray/20 max-w-md">
      <div className="text-center">
        <h2 className="text-xl font-semibold text-text mb-2">
          {t("activation.title", "设备激活")}
        </h2>
        <p className="text-sm text-text/60">
          {t(
            "activation.description",
            "请输入激活码以解锁完整功能",
          )}
        </p>
      </div>

      <div className="w-full">
        <label className="block text-xs font-medium text-text/50 mb-1">
          {t("activation.deviceId", "设备 ID")}
        </label>
        <div className="flex items-center gap-2">
          <code className="flex-1 px-3 py-2 text-xs bg-mid-gray/10 rounded-lg break-all text-text/70 select-all">
            {deviceId || "Loading..."}
          </code>
          <button
            onClick={() => deviceId && navigator.clipboard.writeText(deviceId)}
            className="px-3 py-2 text-xs bg-mid-gray/10 hover:bg-mid-gray/20 rounded-lg transition-colors"
            type="button"
          >
            {t("activation.copy", "复制")}
          </button>
        </div>
      </div>

      <div className="w-full">
        <label className="block text-xs font-medium text-text/50 mb-1">
          {t("activation.code", "激活码")}
        </label>
        <textarea
          value={code}
          onChange={(e) => setCode(e.target.value)}
          placeholder={t("activation.codePlaceholder", "粘贴激活码...")}
          rows={4}
          className="w-full px-3 py-2 text-sm bg-background border border-mid-gray/20 rounded-lg focus:outline-none focus:border-logo-primary resize-none"
        />
      </div>

      {error && (
        <div className="w-full px-3 py-2 text-sm text-red-500 bg-red-500/10 rounded-lg">
          {error}
        </div>
      )}

      <button
        onClick={handleSubmit}
        disabled={loading || !code.trim()}
        className="w-full px-4 py-2.5 text-sm font-medium text-white bg-logo-primary rounded-lg hover:opacity-90 transition-opacity disabled:opacity-50"
      >
        {loading
          ? t("activation.activating", "激活中...")
          : t("activation.activate", "激活")}
      </button>
    </div>
  );
};

export default ActivationGate;