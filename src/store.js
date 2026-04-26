import { createStore } from 'vuex';

// Create a new store instance.
const store = createStore({
  state: {
    Config: {
        X: 0,
        Y: 77,
        Width: 443,
        Height: 887,
        Skin:"light",
        Current_Menu_Id: 1,
        Current_Tags_Id: -1,
        Tools_Order_Field: "number",
        Tools_Order_Type: "desc",
        HotKey_Show_Hidden: "Alt+2",
        HotKey_Search: "Alt+3",
        Startup: true,
        Startup_Background: false,
        Locked_Position: false,
        Locked_Size: false,
        Terminal: "C:\\Windows\\System32\\cmd.exe", // Note the double backslash for escape
        Terminal_Runas_Arguments: "/K cd /D \"{DirectoryPath}\"",
        LeftWidth: 80,
        Top_Window: true,
        Builtin_Tools_OS: "windows",
        Plugin_Http_Host: "127.0.0.1",
        Plugin_Http_Port: 13678,
    },
    Menus:[],
    Tags:[],
    Tools:[],
    isClose: false,
  },
  mutations: {
    updateConfig(state, config) {
      // 使用 Object.assign 合并新旧配置
      Object.assign(state.Config, config);
      // state.Config = config;
    },
    updateConfigLeftWidth(state, width) {
        state.Config.LeftWidth = width;
    },
    updateConfigSize(state,{ width, height }) {
      state.Config.Width = width;
      state.Config.Height = height;
    },
    updateConfigPosition(state, { x, y }) {
      state.Config.X = x;
      state.Config.Y = y;
    },
    updateMenus(state,menus) {
        state.Menus = menus;
    },
    updateTags(state,tags) {
        state.Tags = tags;
    },
    updateTools(state,tools) {
        state.Tools = tools;
    },
    updateOneTools(state,tools) {
  // 遍历 tools 参数中的每个工具
      tools.forEach((newTool) => {
        // 查找 state.Tools 中是否有相同 id 的工具
        const index = state.Tools.findIndex((tool) => tool.id === newTool.id);
        if (index !== -1) {
          // 如果找到，则覆盖原有的工具
          state.Tools[index] = { ...state.Tools[index], ...newTool };
        } else {
          state.Tools.push(newTool);
        }
      });
    },
    updateCurrent_Menu_Id(state,id) {
      state.Config.Current_Menu_Id = id;
    },
    updateCurrent_Tags_Id(state,id) {
      state.Config.Current_Tags_Id = id;
    },
    updateTools_Order(state,{field,type}) {
      state.Config.Tools_Order_Field = field;
      state.Config.Tools_Order_Type = type;
    },
    updateSkin(state,skin) {
      state.Config.Skin = skin;
    },
    updateClose(state,close) {
      state.isClose = close;
    },
    
  },
  actions: {
    // Handle asynchronous logic for state changes
  },
  getters: {
    // Derive some state from the store
  }
});

export default store;
