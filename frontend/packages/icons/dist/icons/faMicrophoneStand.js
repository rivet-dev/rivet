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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faMicrophoneStand.js
var require_faMicrophoneStand = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faMicrophoneStand.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "microphone-stand";
    var width = 512;
    var height = 512;
    var aliases = [127908];
    var unicode = "f8cb";
    var svgPathData = "M479.2 32.8c-43.7-43.7-114.7-43.7-158.4 0l-4.7 4.7L474.5 195.9l4.7-4.7c43.7-43.7 43.7-114.7 0-158.4zM293.5 60.1l-4.7 4.7c-24.4 24.4-35.2 57.2-32.4 89L358.2 255.6c31.8 2.8 64.6-8 89-32.4l4.7-4.7L293.5 60.1zM229.1 171.7L215.8 187 7.8 427c-11 12.7-10.3 31.7 1.6 43.6l32 32c11.9 11.9 30.9 12.6 43.6 1.6L224 383.7l0 96.3c0 17.7 14.3 32 32 32s32-14.3 32-32l0-151.8 37-32 15.3-13.3L229.1 171.7z";
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
    exports.faMicrophoneStand = exports.definition;
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

// src/_entries/faMicrophoneStand.js
var import_faMicrophoneStand = __toESM(require_faMicrophoneStand());
var export_faMicrophoneStand = import_faMicrophoneStand.definition;
export {
  export_faMicrophoneStand as faMicrophoneStand
};
