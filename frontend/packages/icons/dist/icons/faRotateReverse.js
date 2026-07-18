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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faRotateReverse.js
var require_faRotateReverse = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faRotateReverse.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "rotate-reverse";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e631";
    var svgPathData = "M368.9 142.9c17.5 17.5 30.1 38 37.8 59.8c5.9 16.7 24.2 25.4 40.8 19.5s25.4-24.2 19.5-40.8c-10.8-30.6-28.4-59.3-52.9-83.7C327 10.5 185.9 10.1 98.3 96.6L56.7 55c-6.9-6.9-17.2-8.9-26.2-5.2s-14.8 12.5-14.8 22.2l0 128c0 13.3 10.7 24 24 24l8.4 0c0 0 0 0 0 0l119.5 0c9.7 0 18.5-5.8 22.2-14.8s1.7-19.3-5.2-26.2l-41.1-41.1c62.6-61.5 163.1-61.2 225.3 1zM495.8 312c0-13.3-10.7-24-24-24l-7.6 0-.7 0-119.6 0c-9.7 0-18.5 5.8-22.2 14.8s-1.7 19.3 5.2 26.2l41.1 41.1c-62.6 61.5-163.1 61.2-225.3-1c-17.5-17.5-30.1-38-37.8-59.8c-5.9-16.7-24.2-25.4-40.8-19.5s-25.4 24.2-19.5 40.8C55.3 361.3 73 390 97.4 414.4c87.2 87.2 228.3 87.5 315.8 1L454.8 457c6.9 6.9 17.2 8.9 26.2 5.2s14.8-12.5 14.8-22.2l0-119.6 0-.7 0-7.6z";
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
    exports.faRotateReverse = exports.definition;
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

// src/_entries/faRotateReverse.js
var import_faRotateReverse = __toESM(require_faRotateReverse());
var export_faRotateReverse = import_faRotateReverse.definition;
export {
  export_faRotateReverse as faRotateReverse
};
