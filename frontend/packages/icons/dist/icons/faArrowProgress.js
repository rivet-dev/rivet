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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faArrowProgress.js
var require_faArrowProgress = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faArrowProgress.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "arrow-progress";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e5df";
    var svgPathData = "M448 128A64 64 0 1 0 448 0a64 64 0 1 0 0 128zM128 32C57.3 32 0 89.3 0 160s57.3 128 128 128l256 0c35.3 0 64 28.7 64 64c0 27.9-17.9 51.7-42.8 60.4c-11.5-17.1-31-28.4-53.2-28.4c-35.3 0-64 28.7-64 64s28.7 64 64 64c24.7 0 46.1-14 56.8-34.4C467.6 466.1 512 414.2 512 352c0-70.7-57.3-128-128-128l-256 0c-35.3 0-64-28.7-64-64s28.7-64 64-64l128 0 0 14.1c0 9.9 8 17.9 17.9 17.9c4 0 7.8-1.3 11-3.8l60.8-47.3c4-3.1 6.3-7.9 6.3-12.9s-2.3-9.8-6.3-12.9L284.8 3.8c-3.1-2.4-7-3.8-11-3.8C264 0 256 8 256 17.9L256 32 128 32zm-8.6 384c-11.1-19.1-31.7-32-55.4-32c-35.3 0-64 28.7-64 64s28.7 64 64 64c23.7 0 44.4-12.9 55.4-32l40.6 0 0 14.1c0 9.9 8 17.9 17.9 17.9c4 0 7.8-1.3 11-3.8l60.8-47.3c4-3.1 6.3-7.9 6.3-12.9s-2.3-9.8-6.3-12.9l-60.8-47.3c-3.1-2.4-7-3.8-11-3.8c-9.9 0-17.9 8-17.9 17.9l0 14.1-40.6 0z";
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
    exports.faArrowProgress = exports.definition;
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

// src/_entries/faArrowProgress.js
var import_faArrowProgress = __toESM(require_faArrowProgress());
var export_faArrowProgress = import_faArrowProgress.definition;
export {
  export_faArrowProgress as faArrowProgress
};
