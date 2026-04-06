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

// src/node_modules/@fortawesome/free-solid-svg-icons/faHotdog.js
var require_faHotdog = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faHotdog.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "hotdog";
    var width = 512;
    var height = 512;
    var aliases = [127789];
    var unicode = "f80f";
    var svgPathData = "M288 0c-20.5 0-40.1 8.1-54.6 22.6L22.6 233.4C8.1 247.9 0 267.5 0 288 0 300.2 2.9 312.1 8.2 322.7L322.7 8.2C312.1 2.9 300.2 0 288 0zM224 512c20.5 0 40.1-8.1 54.6-22.6L489.4 278.6c14.5-14.5 22.6-34.1 22.6-54.6 0-12.2-2.9-24.1-8.2-34.7L189.3 503.8c10.7 5.4 22.6 8.2 34.7 8.2zM456.6 168.6c31.2-31.2 31.2-81.9 0-113.1s-81.9-31.2-113.1 0l-288 288c-31.2 31.2-31.2 81.9 0 113.1s81.9 31.2 113.1 0l288-288z";
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
    exports.faHotdog = exports.definition;
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

// src/_entries/faHotdog.js
var import_faHotdog = __toESM(require_faHotdog());
var export_faHotdog = import_faHotdog.definition;
export {
  export_faHotdog as faHotdog
};
