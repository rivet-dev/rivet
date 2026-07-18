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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faLampDesk.js
var require_faLampDesk = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faLampDesk.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "lamp-desk";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e014";
    var svgPathData = "M192 65.9C192 29.5 221.5 0 257.9 0c17.5 0 34.3 6.9 46.6 19.3L349.3 64 480 64c12.9 0 24.6 7.8 29.6 19.8s2.2 25.7-6.9 34.9l-73.5 73.5-45.1 45.1-73.5 73.5c-9.2 9.2-22.9 11.9-34.9 6.9s-19.8-16.6-19.8-29.6l0-114.7-92.6 92.6L209 448l143 0c17.7 0 32 14.3 32 32s-14.3 32-32 32l-167.3 0c-.5 0-.9 0-1.4 0L32 512c-17.7 0-32-14.3-32-32s14.3-32 32-32l111 0L97 263.8c-2.7-10.9 .5-22.4 8.4-30.4L218.7 120l-7.4-7.4C198.9 100.2 192 83.4 192 65.9zM432 288c-14.7 0-27.8-6.6-36.6-16.9l67.7-67.7c10.3 8.8 16.9 21.9 16.9 36.6c0 26.5-21.5 48-48 48z";
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
    exports.faLampDesk = exports.definition;
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

// src/_entries/faLampDesk.js
var import_faLampDesk = __toESM(require_faLampDesk());
var export_faLampDesk = import_faLampDesk.definition;
export {
  export_faLampDesk as faLampDesk
};
