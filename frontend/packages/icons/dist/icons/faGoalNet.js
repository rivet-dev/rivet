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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faGoalNet.js
var require_faGoalNet = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faGoalNet.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "goal-net";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e3ab";
    var svgPathData = "M64 128c0-17.7 14.3-32 32-32l448 0c17.7 0 32 14.3 32 32l0 320c0 17.7 14.3 32 32 32s32-14.3 32-32l0-320c0-53-43-96-96-96L96 32C43 32 0 75 0 128L0 448c0 17.7 14.3 32 32 32s32-14.3 32-32l0-320zm112 0c-8.8 0-16 7.2-16 16l0 48-48 0c-8.8 0-16 7.2-16 16s7.2 16 16 16l48 0 0 64-48 0c-8.8 0-16 7.2-16 16s7.2 16 16 16l48 0 0 64-48 0c-8.8 0-16 7.2-16 16s7.2 16 16 16l48 0 0 48c0 8.8 7.2 16 16 16s16-7.2 16-16l0-48 64 0 0 48c0 8.8 7.2 16 16 16s16-7.2 16-16l0-48 64 0 0 48c0 8.8 7.2 16 16 16s16-7.2 16-16l0-48 64 0 0 48c0 8.8 7.2 16 16 16s16-7.2 16-16l0-48 48 0c8.8 0 16-7.2 16-16s-7.2-16-16-16l-48 0 0-64 48 0c8.8 0 16-7.2 16-16s-7.2-16-16-16l-48 0 0-64 48 0c8.8 0 16-7.2 16-16s-7.2-16-16-16l-48 0 0-48c0-8.8-7.2-16-16-16s-16 7.2-16 16l0 48-64 0 0-48c0-8.8-7.2-16-16-16s-16 7.2-16 16l0 48-64 0 0-48c0-8.8-7.2-16-16-16s-16 7.2-16 16l0 48-64 0 0-48c0-8.8-7.2-16-16-16zm80 256l-64 0 0-64 64 0 0 64zm96 0l-64 0 0-64 64 0 0 64zm96 0l-64 0 0-64 64 0 0 64zm0-96l-64 0 0-64 64 0 0 64zm-96 0l-64 0 0-64 64 0 0 64zm-96 0l-64 0 0-64 64 0 0 64z";
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
    exports.faGoalNet = exports.definition;
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

// src/_entries/faGoalNet.js
var import_faGoalNet = __toESM(require_faGoalNet());
var export_faGoalNet = import_faGoalNet.definition;
export {
  export_faGoalNet as faGoalNet
};
