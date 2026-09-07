import { useEffect, useState } from "react"

export type Language = "zh" | "en"

const LANG_EVENT = "cloudcell-lang"

const en = {
  auth: {
    productName: "Cloudcell",
    brandingTitle: "Kernel sandboxes",
    brandingSubtitle: "for AI agents",
    brandingDescription:
      "Run untrusted agent code in AgentCell jails — namespaces, Landlock, seccomp, and cgroup. No Docker.",
    statStart: "cold start",
    statDocker: "containers",
    statKernel: "isolation",
    loginTitle: "Sign in",
    registerTitle: "Create account",
    noAccount: "No account?",
    hasAccount: "Already registered?",
    registerNow: "Register",
    loginNow: "Sign in",
    email: "Email",
    emailPlaceholder: "you@company.com",
    password: "Password",
    passwordPlaceholder: "Enter password",
    confirmPassword: "Confirm password",
    confirmPasswordPlaceholder: "Enter password again",
    login: "Sign in",
    register: "Create account",
    sendCode: "Send code",
    code: "Verification code",
    codePlaceholder: "6-digit code",
    codeSent: "Code sent",
    codeInvalid: "Enter the 6-digit code",
    registerOk: "Account created",
    passwordTooShort: "Password must be at least 8 characters",
    passwordMismatch: "Passwords do not match",
    showPassword: "Show password",
    hidePassword: "Hide password",
    signOut: "Sign out",
  },
  common: {
    error: "Request failed",
    loading: "…",
    language: "Language",
    chinese: "中文",
    english: "English",
  },
}

const zh: typeof en = {
  auth: {
    productName: "Cloudcell",
    brandingTitle: "内核沙箱",
    brandingSubtitle: "给 AI Agent",
    brandingDescription:
      "在 AgentCell jail 里跑不信任的代码：namespace、Landlock、seccomp、cgroup。不用 Docker。",
    statStart: "冷启动",
    statDocker: "容器",
    statKernel: "隔离",
    loginTitle: "登录",
    registerTitle: "创建账号",
    noAccount: "还没有账号？",
    hasAccount: "已经注册？",
    registerNow: "去注册",
    loginNow: "去登录",
    email: "邮箱",
    emailPlaceholder: "you@company.com",
    password: "密码",
    passwordPlaceholder: "输入密码",
    confirmPassword: "确认密码",
    confirmPasswordPlaceholder: "再次输入密码",
    login: "登录",
    register: "创建账号",
    sendCode: "发送验证码",
    code: "验证码",
    codePlaceholder: "6 位验证码",
    codeSent: "验证码已发送",
    codeInvalid: "请输入 6 位验证码",
    registerOk: "账号已创建",
    passwordTooShort: "密码至少 8 位",
    passwordMismatch: "两次密码不一致",
    showPassword: "显示密码",
    hidePassword: "隐藏密码",
    signOut: "退出",
  },
  common: {
    error: "请求失败",
    loading: "…",
    language: "语言",
    chinese: "中文",
    english: "English",
  },
}

const dicts = { en, zh }

function currentLanguage(): Language {
  const language =
    typeof localStorage !== "undefined" ? localStorage.getItem("language") : null
  return language === "zh" || language === "en" ? language : "en"
}

export function useI18n() {
  const [language, setLangState] = useState<Language>(currentLanguage)
  useEffect(() => {
    const onChange = () => setLangState(currentLanguage())
    window.addEventListener(LANG_EVENT, onChange)
    return () => window.removeEventListener(LANG_EVENT, onChange)
  }, [])
  return {
    language,
    setLanguage: (lang: Language) => {
      localStorage.setItem("language", lang)
      window.dispatchEvent(new Event(LANG_EVENT))
    },
  }
}

export function t(key: string, fallback?: string): string {
  const lang = currentLanguage()
  const parts = key.split(".")
  let value: unknown = dicts[lang]
  for (const part of parts) {
    if (value && typeof value === "object" && part in value) {
      value = (value as Record<string, unknown>)[part]
    } else {
      value = undefined
      break
    }
  }
  if (typeof value === "string") return value
  if (fallback && fallback !== "zh" && fallback !== "en") return fallback
  return key
}
