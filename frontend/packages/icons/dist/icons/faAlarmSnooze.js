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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faAlarmSnooze.js
var require_faAlarmSnooze = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faAlarmSnooze.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "alarm-snooze";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f845";
    var svgPathData = "M160 25.4C143 9.6 120.2 0 95.2 0C42.6 0 0 42.6 0 95.2c0 18.8 5.5 36.3 14.9 51.1L160 25.4zM256 512c50.3 0 96.8-16.6 134.2-44.6l35.2 35.2c12.5 12.5 32.8 12.5 45.3 0s12.5-32.8 0-45.3l-35.2-35.2c28-37.4 44.6-83.8 44.6-134.2C480 164.3 379.7 64 256 64S32 164.3 32 288c0 50.3 16.6 96.8 44.6 134.2L41.4 457.4c-12.5 12.5-12.5 32.8 0 45.3s32.8 12.5 45.3 0l35.2-35.2c37.4 28 83.8 44.6 134.1 44.6zM497.1 146.4C506.5 131.6 512 114 512 95.2C512 42.6 469.4 0 416.8 0C391.8 0 369 9.6 352 25.4L497.1 146.4zM200 192l112 0c9.2 0 17.5 5.2 21.6 13.5s3 18-2.6 25.3L249.1 336l62.9 0c13.3 0 24 10.7 24 24s-10.7 24-24 24l-112 0c-9.2 0-17.5-5.2-21.6-13.5s-3-18 2.6-25.3L262.9 240 200 240c-13.3 0-24-10.7-24-24s10.7-24 24-24z";
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
    exports.faAlarmSnooze = exports.definition;
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

// src/_entries/faAlarmSnooze.js
var import_faAlarmSnooze = __toESM(require_faAlarmSnooze());
var export_faAlarmSnooze = import_faAlarmSnooze.definition;
export {
  export_faAlarmSnooze as faAlarmSnooze
};
