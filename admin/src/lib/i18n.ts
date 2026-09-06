export type Language = "zh" | "en"

const en = {
  auth: {
    introBadge: "Sandbox platform",
    introHeading: "Kernel sandboxes for AI agents",
    introSubheading:
      "Run untrusted agent code in AgentCell jails — namespaces, Landlock, seccomp, and cgroup. No Docker.",
    introPoint1Title: "Fast cells",
    introPoint1Desc: "sand serve starts in milliseconds. Exec rides the unix protocol.",
    introPoint2Title: "Kernel isolation",
    introPoint2Desc: "User, mount, pid, and net namespaces with Landlock and seccomp.",
    introPoint3Title: "API first",
    introPoint3Desc: "Create sandboxes, keys, and exec from the console or HTTP.",
    tagGit: "AgentCell",
    tagSession: "cgroup v2",
    tagLoop: "Landlock",
    registerTab: "Register",
    loginTab: "Sign in",
    workspaceLoginTitle: "Sign in to Cloudcell",
    workspaceLoginDesc: "Use the email and password for this workspace.",
    workspaceRegisterTitle: "Create your account",
    workspaceRegisterDesc: "Verify email, then set a username and password.",
    email: "Email",
    password: "Password",
    confirmPassword: "Confirm password",
    username: "Username",
    usernamePlaceholder: "workspace-name",
    login: "Sign in",
    sendCode: "Send code",
    resendCode: "Resend",
    code: "Verification code",
    codeSent: "Code sent",
    codeInvalid: "Enter the 6-digit code",
    next: "Next",
    back: "Back",
    completeRegister: "Create account",
    registerOk: "Account created",
    passwordTooShort: "Password must be at least 8 characters",
    passwordMismatch: "Passwords do not match",
    termsNotice: "By continuing you agree to the Cloudcell terms of use.",
    signOut: "Sign out",
  },
  common: { error: "Request failed" },
}

const zh: typeof en = {
  auth: {
    introBadge: "沙箱平台",
    introHeading: "给 AI Agent 用的内核沙箱",
    introSubheading:
      "在 AgentCell jail 里跑不信任的代码：namespace、Landlock、seccomp、cgroup。不用 Docker。",
    introPoint1Title: "启动快",
    introPoint1Desc: "sand serve 毫秒级拉起，exec 走 unix 协议。",
    introPoint2Title: "内核隔离",
    introPoint2Desc: "user / mount / pid / net namespace，加上 Landlock 与 seccomp。",
    introPoint3Title: "API 优先",
    introPoint3Desc: "控制台或 HTTP 创建 sandbox、密钥并执行命令。",
    tagGit: "AgentCell",
    tagSession: "cgroup v2",
    tagLoop: "Landlock",
    registerTab: "注册",
    loginTab: "登录",
    workspaceLoginTitle: "登录 Cloudcell",
    workspaceLoginDesc: "使用此工作区的邮箱和密码。",
    workspaceRegisterTitle: "创建账号",
    workspaceRegisterDesc: "先验证邮箱，再设置用户名和密码。",
    email: "邮箱",
    password: "密码",
    confirmPassword: "确认密码",
    username: "用户名",
    usernamePlaceholder: "工作区名称",
    login: "登录",
    sendCode: "发送验证码",
    resendCode: "重新发送",
    code: "验证码",
    codeSent: "验证码已发送",
    codeInvalid: "请输入 6 位验证码",
    next: "下一步",
    back: "返回",
    completeRegister: "完成注册",
    registerOk: "账号已创建",
    passwordTooShort: "密码至少 8 位",
    passwordMismatch: "两次密码不一致",
    termsNotice: "继续即表示同意 Cloudcell 使用条款。",
    signOut: "退出",
  },
  common: { error: "请求失败" },
}

const dicts = { en, zh }

export function useI18n() {
  const language = (
    typeof localStorage !== "undefined" ? localStorage.getItem("language") : null
  ) as Language | null
  return {
    language: language === "zh" || language === "en" ? language : ("en" as const),
    setLanguage: (lang: Language) => {
      localStorage.setItem("language", lang)
    },
  }
}

function currentLanguage(): Language {
  const language =
    typeof localStorage !== "undefined" ? localStorage.getItem("language") : null
  return language === "zh" || language === "en" ? language : "en"
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
