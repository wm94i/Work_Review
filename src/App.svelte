<script>
  import { onMount } from 'svelte';
  import Router from 'svelte-spa-router';
  import Sidebar from './lib/components/Sidebar.svelte';
  import Toast from './lib/components/Toast.svelte';
  import Overview from './routes/Overview.svelte';
  import Timeline from './routes/timeline/Timeline.svelte';
  import Summary from './routes/timeline/Summary.svelte';
  import Report from './routes/report/Report.svelte';
  import Settings from './routes/settings/Settings.svelte';
  import About from './routes/about/About.svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
  import { cache, getLocalDate } from './lib/stores/cache.js';
  import { runUpdateFlow } from './lib/utils/updater.js';

  const appWindow = getCurrentWebviewWindow();

  // 窗口控制函数
  async function closeWindow() {
    await appWindow.hide();
  }

  async function minimizeWindow() {
    await appWindow.minimize();
  }

  async function maximizeWindow() {
    const isMaximized = await appWindow.isMaximized();
    if (isMaximized) {
      await appWindow.unmaximize();
    } else {
      await appWindow.maximize();
    }
  }

  // 预加载核心数据
  async function preloadApp() {
    console.log('开始预加载数据...');
    const today = getLocalDate();
    
    // 并行预加载：概览、时间线(今天)、日报(今天)
    Promise.all([
      // 1. 概览
      invoke('get_today_stats').then(stats => cache.setOverview(stats)),
      // 2. 时间线 (今天) - 仅预加载前 20 条
      Promise.all([
        invoke('get_timeline', { date: today, limit: 20, offset: 0 }),
        invoke('get_hourly_summaries', { date: today })
      ]).then(([activities, summaries]) => cache.setTimeline(today, activities, summaries)),
      // 3. 日报 (今天) - 检查是否已存在
      invoke('get_saved_report', { date: today }).then(report => {
        if (report) cache.setReport(today, report);
      })
    ]).then(() => {
      console.log('预加载完成');
    }).catch(e => {
      console.warn('预加载部分失败:', e);
    });
  }

  const routes = {
    '/': Overview,
    '/timeline': Timeline,
    '/timeline/summary': Summary,
    '/report': Report,
    '/settings': Settings,
    '/about': About,
  };

  let theme = 'system';
  let isDark = false;
  let isRecording = true;
  let isPaused = false;
  let platform = '';
  let backgroundImage = null;
  let backgroundOpacity = 0.25;
  let backgroundBlur = 1;
  let runtimeConfig = null;

  function detectSystemTheme() {
    return window.matchMedia('(prefers-color-scheme: dark)').matches;
  }

  function applyTheme(newTheme) {
    theme = newTheme;
    isDark = theme === 'system' ? detectSystemTheme() : theme === 'dark';
    
    if (isDark) {
      document.documentElement.classList.add('dark');
    } else {
      document.documentElement.classList.remove('dark');
    }
  }

  async function handleThemeChange(event) {
    const newTheme = event.detail;
    applyTheme(newTheme);

    try {
      const config = await invoke('get_config');
      config.theme = newTheme;
      await invoke('save_config', { config });
      cache.setConfig(config);
    } catch (e) {
      console.error('保存主题配置失败:', e);
    }
  }

  async function loadBackground() {
    try {
      const config = await invoke('get_config');
      backgroundOpacity = config.background_opacity ?? 0.25;
      backgroundBlur = config.background_blur ?? 1;
      if (config.background_image) {
        const b64 = await invoke('get_background_image');
        if (b64) {
          backgroundImage = `data:image/jpeg;base64,${b64}`;
        }
      } else {
        backgroundImage = null;
      }
    } catch (e) {
      console.warn('加载背景图失败:', e);
    }
  }

  // 实时响应设置页的背景参数变更（不需要保存即可生效）
  function handleBackgroundChanged(e) {
    const d = e.detail;
    if (d) {
      if (d.image !== undefined) backgroundImage = d.image;
      if (d.opacity !== undefined) backgroundOpacity = d.opacity;
      if (d.blur !== undefined) backgroundBlur = d.blur;
    }
  }

  onMount(async () => {
    // 获取平台信息
    try {
      platform = await invoke('get_platform');
      console.log('当前平台:', platform);
    } catch (e) {
      console.error('获取平台信息失败:', e);
    }

    // 加载配置并应用主题
    let config;
    try {
      config = await invoke('get_config');
      runtimeConfig = config;
      cache.setConfig(config);
      applyTheme(config.theme || 'system');
    } catch (e) {
      console.error('加载配置失败:', e);
      applyTheme('system');
      config = { work_end_hour: 18 };
      runtimeConfig = config;
    }

    // 加载背景图
    loadBackground();

    try {
      const [recording, paused] = await invoke('get_recording_state');
      isRecording = recording;
      isPaused = paused;
    } catch (e) {
      console.error('获取录制状态失败:', e);
    }

    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
    const handleSystemThemeChange = () => {
      if (theme === 'system') applyTheme('system');
    };
    mediaQuery.addEventListener('change', handleSystemThemeChange);

    const unsubscribeCache = cache.subscribe((state) => {
      if (!state.config) return;
      runtimeConfig = state.config;

      if (state.config.theme && state.config.theme !== theme) {
        applyTheme(state.config.theme);
      }
    });

    // 监听背景图更新事件（来自设置页，实时预览）
    const handleBgChange = (e) => handleBackgroundChanged(e);
    window.addEventListener('background-changed', handleBgChange);

    // 启动预加载
    preloadApp();

    // 启动后延迟执行一次自动更新检查，避免阻塞首屏渲染
    const autoUpdateTimer = setTimeout(async () => {
      try {
        const shouldCheck = await invoke('should_check_updates');
        if (!shouldCheck) return;

        await invoke('update_last_check_time');
        await runUpdateFlow({
          silentWhenUpToDate: true,
          confirmBeforeDownload: true,
          onStatusChange: () => {},
        });
      } catch (e) {
        console.warn('自动检查更新失败:', e);
      }
    }, 2000);

    // 日报自动生成检测：每分钟检查一次
    let lastAutoGenDate = null;  // 防止同一天重复触发
    const autoReportTimer = setInterval(async () => {
      const now = new Date();
      const currentHour = now.getHours();
      const currentMinute = now.getMinutes();
      const today = getLocalDate();

      // 检查是否达到工作结束时间
      const workEndHour = runtimeConfig?.work_end_hour ?? 18;
      const workEndMinute = runtimeConfig?.work_end_minute ?? 0;

      // 条件：当前小时等于工作结束时间，当前分钟 >= 结束分钟，且今天未自动生成过
      if (currentHour === workEndHour && currentMinute >= workEndMinute && lastAutoGenDate !== today) {
        try {
          // 检查今日是否已有日报
          const existingReport = await invoke('get_saved_report', { date: today });
          if (!existingReport) {
            console.log('工作结束时间到达，自动生成日报...');
            await invoke('generate_report', { date: today, force: false });
            cache.invalidate('report', today);
            lastAutoGenDate = today;
            console.log('日报自动生成完成');
          } else {
            lastAutoGenDate = today;  // 已有日报，标记今天不再触发
          }
        } catch (e) {
          console.warn('日报自动生成失败:', e);
        }
      }
    }, 60000);  // 每分钟检查一次

    const unlisten = await listen('screenshot-taken', (event) => {
      console.log('截屏完成:', event.payload);
      
      // 1. 增量更新时间线缓存
      cache.addActivity(event.payload);
      
      // 2. 使概览缓存过期（下次访问或当前页面监听时刷新）
      cache.invalidate('overview');
      
      // 3. 发射自定义事件，通知当前页面实时更新
      window.dispatchEvent(new CustomEvent('activity-added', { detail: event.payload }));
    });

    return () => {
      unlisten();
      unsubscribeCache();
      clearTimeout(autoUpdateTimer);
      clearInterval(autoReportTimer);
      mediaQuery.removeEventListener('change', handleSystemThemeChange);
      window.removeEventListener('background-changed', handleBgChange);
    };
  });
</script>

<div class="flex h-screen bg-slate-50 dark:bg-slate-900 overflow-hidden relative">
  <!-- 背景图层：图片全强度 + 半透明遮罩控制显隐 -->
  {#if backgroundImage}
    <div class="absolute inset-0 z-0 overflow-hidden pointer-events-none">
      <!-- 图片（全强度，不用 opacity 避免色彩发白） -->
      <div
        class="absolute inset-[-20px] bg-cover bg-center bg-no-repeat"
        style="background-image: url({backgroundImage}); filter: blur({backgroundBlur === 0 ? 0 : backgroundBlur === 1 ? 8 : 16}px);"
      ></div>
      <!-- 半透明遮罩：遮罩越透明 = 背景图越明显 -->
      <div
        class="absolute inset-0 bg-slate-50 dark:bg-slate-900 transition-opacity duration-300"
        style="opacity: {Math.max(0, 1 - backgroundOpacity)};"
      ></div>
    </div>
  {/if}

  <!--
    全局顶部拖拽层 (Invisible Drag Layer)
    1. 覆盖在所有内容之上 (z-50)
    2. 负责处理窗口拖动 (-webkit-app-region: drag)
    3. 按钮区域排除拖动 (-webkit-app-region: no-drag)
  -->
  <div class="absolute top-0 left-0 w-full h-8 z-50" style="-webkit-app-region: drag;">
    <!-- 仅 Windows/Linux 平台显示自定义窗口控制按钮，macOS 使用原生控件 -->
    {#if platform && platform !== 'macos'}
    <!-- Windows 风格窗口控制按钮 (右上角) -->
    <div class="absolute right-0 top-0 flex items-stretch h-8">
      <!-- Minimize -->
      <button
        on:click={minimizeWindow}
        class="w-11 h-full flex items-center justify-center hover:bg-slate-200 dark:hover:bg-slate-700 focus:outline-none transition-colors"
        style="-webkit-app-region: no-drag;"
        title="最小化"
      >
        <svg class="w-3 h-3 text-slate-600 dark:text-slate-300" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
          <path stroke-linecap="round" stroke-linejoin="round" d="M5 12h14" />
        </svg>
      </button>

      <!-- Maximize -->
      <button
        on:click={maximizeWindow}
        class="w-11 h-full flex items-center justify-center hover:bg-slate-200 dark:hover:bg-slate-700 focus:outline-none transition-colors"
        style="-webkit-app-region: no-drag;"
        title="最大化"
      >
        <svg class="w-3 h-3 text-slate-600 dark:text-slate-300" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
          <rect x="4" y="4" width="16" height="16" rx="1" />
        </svg>
      </button>

      <!-- Close -->
      <button
        on:click={closeWindow}
        class="w-11 h-full flex items-center justify-center hover:bg-red-500 hover:text-white focus:outline-none transition-colors group"
        style="-webkit-app-region: no-drag;"
        title="关闭"
      >
        <svg class="w-3 h-3 text-slate-600 dark:text-slate-300 group-hover:text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
          <path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" />
        </svg>
      </button>
    </div>
    {/if}
  </div>

  <!-- 左侧边栏 -->
  <div class="w-56 bg-white/80 dark:bg-slate-900/90 backdrop-blur-xl border-r border-slate-200/50 dark:border-slate-700/50 flex flex-col z-10"
       class:pt-5={platform === 'macos'}
       class:pt-2={platform !== 'macos'}>
    <Sidebar {isRecording} {isPaused} {theme} on:themeChange={handleThemeChange} />
  </div>

  <!-- 右侧主内容区域 -->
  <div class="flex-1 flex flex-col overflow-hidden z-10"
       class:pt-8={platform !== 'macos'}>
    <main class="flex-1 overflow-auto">
      <Router {routes} />
    </main>
  </div>

  <Toast />
</div>
