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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faTeddyBear.js
var require_faTeddyBear = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faTeddyBear.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "teddy-bear";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e3cf";
    var svgPathData = "M154.7 103.7C138.7 93.9 128 76.2 128 56c0-30.9 25.1-56 56-56c24.2 0 44.7 15.3 52.6 36.7C252.5 33.5 269.8 32 288 32s35.5 1.5 51.4 4.7C347.3 15.3 367.8 0 392 0c30.9 0 56 25.1 56 56c0 20.2-10.7 37.9-26.7 47.7c1.8 7.4 2.7 15.5 2.7 24.3c0 70.7-60.9 128-136 128s-136-57.3-136-128c0-8.8 .9-16.8 2.7-24.3zM240 128a16 16 0 1 0 0-32 16 16 0 1 0 0 32zm112-16a16 16 0 1 0 -32 0 16 16 0 1 0 32 0zm0 64c0-26.5-28.7-48-64-48s-64 21.5-64 48s28.7 48 64 48s64-21.5 64-48zm-32-8c0 13.3-14.3 24-32 24s-32-10.7-32-24s14.3-24 32-24s32 10.7 32 24zM50.3 178.3c20.8-16.4 51-12.8 67.4 8L143.5 219c73.7 93.5 215.4 93.5 289.1 0l25.8-32.7c16.4-20.8 46.6-24.4 67.4-8s24.4 46.6 8 67.4l-25.8 32.7c-17.7 22.4-37.9 41.3-59.9 56.7l0 64.9 16.6 0c3.9-27.1 27.2-48 55.4-48c30.9 0 56 25.1 56 56c0 2.7-.2 5.4-.6 8l-13 53.3c-5.8 23.8-26.3 40.9-50.4 42.6l0 .1-4 0-28 0L96 512l-28 0-4 0 0-.1c-24.1-1.7-44.6-18.8-50.4-42.6L.6 416c-.4-2.6-.6-5.3-.6-8c0-30.9 25.1-56 56-56c28.2 0 51.6 20.9 55.4 48l16.6 0 0-64.9c-22-15.4-42.3-34.3-59.9-56.7L42.3 245.7c-16.4-20.8-12.8-51 8-67.4zM256 352c-35.3 0-64 28.7-64 64l192 0c0-35.3-28.7-64-64-64l-64 0z";
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
    exports.faTeddyBear = exports.definition;
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

// src/_entries/faTeddyBear.js
var import_faTeddyBear = __toESM(require_faTeddyBear());
var export_faTeddyBear = import_faTeddyBear.definition;
export {
  export_faTeddyBear as faTeddyBear
};
