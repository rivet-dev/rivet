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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faAlarmExclamation.js
var require_faAlarmExclamation = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faAlarmExclamation.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "alarm-exclamation";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f843";
    var svgPathData = "M14.9 146.4C5.5 131.6 0 114 0 95.2C0 42.6 42.6 0 95.2 0c25 0 47.8 9.6 64.8 25.4L14.9 146.4zM390.2 467.4C352.8 495.4 306.3 512 256 512s-96.8-16.6-134.1-44.6L86.6 502.6c-12.5 12.5-32.8 12.5-45.3 0s-12.5-32.8 0-45.3l35.2-35.2C48.6 384.8 32 338.3 32 288C32 164.3 132.3 64 256 64s224 100.3 224 224c0 50.3-16.6 96.8-44.6 134.2l35.2 35.2c12.5 12.5 12.5 32.8 0 45.3s-32.8 12.5-45.3 0l-35.2-35.2zM352 25.4C369 9.6 391.8 0 416.8 0C469.4 0 512 42.6 512 95.2c0 18.8-5.5 36.3-14.9 51.1L352 25.4zM288 384a32 32 0 1 0 -64 0 32 32 0 1 0 64 0zm-8-200c0-13.3-10.7-24-24-24s-24 10.7-24 24l0 112c0 13.3 10.7 24 24 24s24-10.7 24-24l0-112z";
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
    exports.faAlarmExclamation = exports.definition;
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

// src/_entries/faAlarmExclamation.js
var import_faAlarmExclamation = __toESM(require_faAlarmExclamation());
var export_faAlarmExclamation = import_faAlarmExclamation.definition;
export {
  export_faAlarmExclamation as faAlarmExclamation
};
