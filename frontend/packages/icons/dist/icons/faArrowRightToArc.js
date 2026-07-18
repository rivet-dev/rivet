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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faArrowRightToArc.js
var require_faArrowRightToArc = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faArrowRightToArc.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "arrow-right-to-arc";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e4b2";
    var svgPathData = "M448 256c0-106-86-192-192-192c-17.7 0-32-14.3-32-32s14.3-32 32-32C397.4 0 512 114.6 512 256s-114.6 256-256 256c-17.7 0-32-14.3-32-32s14.3-32 32-32c106 0 192-86 192-192zM230.6 121.4l112 112c12.5 12.5 12.5 32.8 0 45.3l-112 112c-12.5 12.5-32.8 12.5-45.3 0s-12.5-32.8 0-45.3L242.7 288 32 288c-17.7 0-32-14.3-32-32s14.3-32 32-32l210.7 0-57.4-57.4c-12.5-12.5-12.5-32.8 0-45.3s32.8-12.5 45.3 0z";
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
    exports.faArrowRightToArc = exports.definition;
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

// src/_entries/faArrowRightToArc.js
var import_faArrowRightToArc = __toESM(require_faArrowRightToArc());
var export_faArrowRightToArc = import_faArrowRightToArc.definition;
export {
  export_faArrowRightToArc as faArrowRightToArc
};
