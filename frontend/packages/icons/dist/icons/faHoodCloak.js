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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faHoodCloak.js
var require_faHoodCloak = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faHoodCloak.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "hood-cloak";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "f6ef";
    var svgPathData = "M432 0L288 0s0 0 0 0c-9.8 0-66 0-120.2 37.7C111.4 76.9 64 151.8 64 288c0 58.5-14.6 101.6-28.6 129.7c-7 14.1-14 24.5-19 31.1c-2.5 3.3-4.5 5.7-5.7 7.1c-.6 .7-1.1 1.2-1.3 1.4l-.2 .2c-9 9.1-11.7 22.8-6.8 34.7c5 12 16.6 19.8 29.6 19.8l96 0 0-192c0-88.4 71.6-160 160-160s160 71.6 160 160l0 192 96 0c12.9 0 24.6-7.8 29.6-19.8c4.9-11.9 2.2-25.6-6.8-34.7c0 0 0 0 0 0s0 0 0 0s0 0 0 0l-.1-.1c-.2-.2-.7-.7-1.3-1.4c-1.3-1.4-3.2-3.8-5.7-7.1c-5-6.6-11.9-17-19-31.1C526.6 389.6 512 346.5 512 288c0-103.1-46.5-172.8-95.8-217.2l32-29c7.3-6.7 9.8-17.2 6.2-26.4S441.9 0 432 0zM416 512l0-192c0-70.7-57.3-128-128-128s-128 57.3-128 128l0 192 256 0z";
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
    exports.faHoodCloak = exports.definition;
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

// src/_entries/faHoodCloak.js
var import_faHoodCloak = __toESM(require_faHoodCloak());
var export_faHoodCloak = import_faHoodCloak.definition;
export {
  export_faHoodCloak as faHoodCloak
};
