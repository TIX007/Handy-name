/**
 * 前端模型过滤模块
 *
 * 从上游 `useModelStore` 获取模型列表后，
 * 根据激活状态和授权等级过滤可见模型。
 *
 * SenseVoice（系统控制模型）不在模型选择下拉菜单中显示，
 * 由后端根据 Premium 授权自动加载。
 */

import { commands } from "@/bindings";

/** 系统控制模型 ID 列表 */
export const SYSTEM_CONTROLLED_MODELS = ["sense-voice-int8"];

/** 授权等级 */
export type LicenseTier = "free" | "standard" | "premium";

/** 激活状态信息 */
export interface ActivationInfo {
  isActivated: boolean;
  tier: LicenseTier;
  deviceId: string;
  expireAt: string | null;
}

/**
 * 从模型列表中过滤掉用户不应手动选择的模型
 *
 * - 过滤掉所有系统控制模型（SenseVoice 等）
 * - 这些模型由后端根据授权等级自动管理
 */
export function filterUserSelectableModels<T extends { id: string }>(
  models: T[],
): T[] {
  return models.filter((m) => !SYSTEM_CONTROLLED_MODELS.includes(m.id));
}

/**
 * 判断模型是否由系统控制
 */
export function isSystemControlled(modelId: string): boolean {
  return SYSTEM_CONTROLLED_MODELS.includes(modelId);
}

/**
 * 从后端获取当前激活状态和授权等级
 *
 * 降级策略：获取失败时返回未激活状态
 */
export async function getActivationStatus(): Promise<ActivationInfo> {
  try {
    const result = await commands.cnGetActivationStatus();
    if (result.status === "ok") {
      return result.data;
    }
  } catch {
    // 降级
  }
  return {
    isActivated: false,
    tier: "free" as LicenseTier,
    deviceId: "",
    expireAt: null,
  };
}