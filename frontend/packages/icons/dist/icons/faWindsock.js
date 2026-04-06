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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faWindsock.js
var require_faWindsock = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faWindsock.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "windsock";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f777";
    var svgPathData = "M64 32C64 14.3 49.7 0 32 0S0 14.3 0 32L0 480c0 17.7 14.3 32 32 32s32-14.3 32-32l0-160 64 0 0 8.6c0 20.3 18.7 35.5 38.5 31.3l41.5-8.6 0-286.7L166.5 56c-19.9-4.1-38.5 11-38.5 31.3l0 8.6L64 96l0-64zm64 112l0 128-64 0 0-128 64 0zM272 78l0 260 96-20 0-220L272 78zM486.5 293.3c14.8-3.1 25.5-16.2 25.5-31.3l0-108c0-15.2-10.6-28.2-25.5-31.3L432 111.3l0 193.3 54.5-11.4z";
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
    exports.faWindsock = exports.definition;
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

// src/_entries/faWindsock.js
var import_faWindsock = __toESM(require_faWindsock());
var export_faWindsock = import_faWindsock.definition;
export {
  export_faWindsock as faWindsock
};
