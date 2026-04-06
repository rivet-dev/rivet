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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faLightbulbCfl.js
var require_faLightbulbCfl = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faLightbulbCfl.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "lightbulb-cfl";
    var width = 320;
    var height = 512;
    var aliases = [];
    var unicode = "e5a6";
    var svgPathData = "M167.8 63c17.1-4.3 27.6-21.7 23.3-38.8S169.4-3.3 152.2 1L24.2 33C7.1 37.2-3.3 54.6 1 71.8S22.6 99.3 39.8 95l128-32zM319 88.2c-4.3-17.1-21.7-27.6-38.8-23.3l-256 64C7.1 133.2-3.3 150.6 1 167.8s21.7 27.6 38.8 23.3l256-64c17.1-4.3 27.6-21.7 23.3-38.8zM295.8 223c17.1-4.3 27.6-21.7 23.3-38.8s-21.7-27.6-38.8-23.3l-256 64C7.1 229.2-3.3 246.6 1 263.8s21.7 27.6 38.8 23.3l256-64zM64 416l0 32c0 17.7 14.3 32 32 32l32 0c0 17.7 14.3 32 32 32s32-14.3 32-32l32 0c17.7 0 32-14.3 32-32l0-32c0-17.7-14.3-32-32-32l-80 0 0-90L96 306l0 78c-17.7 0-32 14.3-32 32zM319 280.2c-4.3-17.1-21.7-27.6-38.8-23.3l-64 16-16 4c-13.2 3.3-22.4 14.3-24 27l-.2 0 0 3.3c0 .5 0 .9 0 1.4l0 75.3 48 0 0-47 7.8-1.9 64-16c17.1-4.3 27.6-21.7 23.3-38.8z";
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
    exports.faLightbulbCfl = exports.definition;
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

// src/_entries/faLightbulbCfl.js
var import_faLightbulbCfl = __toESM(require_faLightbulbCfl());
var export_faLightbulbCfl = import_faLightbulbCfl.definition;
export {
  export_faLightbulbCfl as faLightbulbCfl
};
