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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faPersonSign.js
var require_faPersonSign = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faPersonSign.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "person-sign";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f757";
    var svgPathData = "M438.1 32.2c4.5-12.5-1.9-26.2-14.3-30.8S397.5 3.3 393 15.8l-4.6 12.7L328.7 6.7c-12.5-4.5-26.2 1.9-30.8 14.3L259.6 126.3c-4.5 12.5 1.9 26.2 14.3 30.8l59.7 21.7-15.4 42.4-28.8-10.8c-9.7-3.7-18.4-9.6-25.4-17.4L235 160.2c-18.2-20.5-44.3-32.2-71.8-32.2l-27.7 0c-36.4 0-69.6 20.5-85.9 53.1L3.4 273.7c-7.9 15.8-1.5 35 14.3 42.9s35 1.5 42.9-14.3L80 263.6 80 480c0 17.7 14.3 32 32 32s32-14.3 32-32l0-128 20.2 0L192 407.6l0 72.4c0 17.7 14.3 32 32 32s32-14.3 32-32l0-72.4c0-9.9-2.3-19.7-6.8-28.6L216 312.4l0-77.3 .3 .3C230.1 251 247.5 263 267 270.3l29.3 11L289 301.5c-4.5 12.5 1.9 26.2 14.3 30.8s26.2-1.9 30.8-14.3l44.6-122.6 60.6 22c12.5 4.5 26.2-1.9 30.8-14.3L508.4 97.7c4.5-12.5-1.9-26.2-14.3-30.8l-60.6-22 4.6-12.7zM192 48A48 48 0 1 0 96 48a48 48 0 1 0 96 0z";
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
    exports.faPersonSign = exports.definition;
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

// src/_entries/faPersonSign.js
var import_faPersonSign = __toESM(require_faPersonSign());
var export_faPersonSign = import_faPersonSign.definition;
export {
  export_faPersonSign as faPersonSign
};
