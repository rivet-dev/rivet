var __create = Object.create;
var __defProp = Object.defineProperty;
var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __getProtoOf = Object.getPrototypeOf;
var __hasOwnProp = Object.prototype.hasOwnProperty;
var __commonJS = (cb, mod) => function __require() {
  return mod || (0, cb[__getOwnPropNames(cb)[0]])((mod = { exports: {} }).exports, mod), mod.exports;
};
var __copyProps = (to, from, except, desc) => {
  if (from && typeof from === "object" || typeof from === "function") {
    for (let key of __getOwnPropNames(from))
      if (!__hasOwnProp.call(to, key) && key !== except)
        __defProp(to, key, { get: () => from[key], enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable });
  }
  return to;
};
var __toESM = (mod, isNodeMode, target) => (target = mod != null ? __create(__getProtoOf(mod)) : {}, __copyProps(
  // If the importer is in node compatibility mode or this is not an ESM
  // file that has been converted to a CommonJS file using a Babel-
  // compatible transform (i.e. "__esModule" has not been set), then set
  // "default" to the CommonJS "module.exports" for node compatibility.
  isNodeMode || !mod || !mod.__esModule ? __defProp(target, "default", { value: mod, enumerable: true }) : target,
  mod
));

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBinRecycle.js
var require_faBinRecycle = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBinRecycle.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "bin-recycle";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e5f7";
    var svgPathData = "M0 64C0 46.3 14.3 32 32 32l512 0c17.7 0 32 14.3 32 32s-14.3 32-32 32L32 96C14.3 96 0 81.7 0 64zm32 64l512 0L517.3 421.8c-3 33-30.6 58.2-63.7 58.2l-331.1 0c-33.1 0-60.7-25.2-63.7-58.2L32 128zm256 88c2.2 0 4.3 1.1 5.5 2.9l20.7 29.6c7.3 10.5 21.6 13.4 32.4 6.6c11.7-7.3 14.8-22.9 6.9-34.1l-20.7-29.6c-10.2-14.6-27-23.3-44.8-23.3s-34.6 8.7-44.8 23.3l-20.7 29.6c-7.9 11.3-4.7 26.8 6.9 34.1c10.8 6.8 25.1 3.9 32.4-6.6l20.7-29.6c1.3-1.8 3.3-2.9 5.5-2.9zm-88.3 77.1c-10.8-6.8-25.1-3.9-32.4 6.6l-21.5 30.7c-6.4 9.1-9.8 20-9.8 31.2c0 30.1 24.4 54.4 54.4 54.4l49.6 0c13.3 0 24-10.7 24-24s-10.7-24-24-24l-49.6 0c-3.5 0-6.4-2.9-6.4-6.4c0-1.3 .4-2.6 1.2-3.7l21.5-30.7c7.9-11.3 4.7-26.8-6.9-34.1zM312 392c0 13.3 10.7 24 24 24l49.6 0c30.1 0 54.4-24.4 54.4-54.4c0-11.2-3.4-22.1-9.8-31.2l-21.5-30.7c-7.3-10.5-21.6-13.4-32.4-6.6c-11.7 7.3-14.8 22.9-6.9 34.1l21.5 30.7c.8 1.1 1.2 2.4 1.2 3.7c0 3.5-2.9 6.4-6.4 6.4L336 368c-13.3 0-24 10.7-24 24z";
    exports.definition = {
      prefix,
      iconName,
      icon: [
        width,
        height,
        aliases,
        unicode,
        svgPathData
      ]
    };
    exports.faBinRecycle = exports.definition;
    exports.prefix = prefix;
    exports.iconName = iconName;
    exports.width = width;
    exports.height = height;
    exports.ligatures = aliases;
    exports.unicode = unicode;
    exports.svgPathData = svgPathData;
    exports.aliases = aliases;
  }
});

// src/_entries/faBinRecycle.js
var import_faBinRecycle = __toESM(require_faBinRecycle());
var export_faBinRecycle = import_faBinRecycle.definition;
export {
  export_faBinRecycle as faBinRecycle
};
