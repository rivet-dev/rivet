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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCrutches.js
var require_faCrutches = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCrutches.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "crutches";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f7f8";
    var svgPathData = "M470.6 9.4l160 160c12.5 12.5 12.5 32.8 0 45.3s-32.8 12.5-45.3 0l-160-160c-12.5-12.5-12.5-32.8 0-45.3s32.8-12.5 45.3 0zM294.6 188.2L404.1 78.6l45.3 45.3L397.3 176 464 242.7l52.1-52.1 45.3 45.3-74.6 74.6-.1 .1-.1 .1-34.7 34.7c-12.9 12.9-29.2 21.9-47.1 25.8L297.1 395.2c-5.9 1.3-11.4 4.3-15.7 8.6l-98.8 98.8c-12.5 12.5-32.8 12.5-45.3 0s-12.5-32.8 0-45.3l98.8-98.8c4.3-4.3 7.3-9.7 8.6-15.7l23.9-107.6c4-17.8 12.9-34.1 25.8-47.1zm45.3 45.3c-4.3 4.3-7.3 9.7-8.6 15.7l-17 76.7 76.7-17c5.9-1.3 11.4-4.3 15.7-8.6L418.7 288 352 221.3l-12.2 12.2zM169.4 9.4c12.5-12.5 32.8-12.5 45.3 0s12.5 32.8 0 45.3L213.3 56l84.1 84.1-25.4 25.4c-6.7 6.7-12.6 14.1-17.6 22L168 101.3 101.3 168 223.7 290.4l-10.1 45.5-17 17c-2.9-2.3-5.7-4.8-8.4-7.5L56 213.3l-1.4 1.4c-12.5 12.5-32.8 12.5-45.3 0s-12.5-32.8 0-45.3l160-160zM411.7 402.5c9.3-2.1 18.3-5.2 26.9-9.2l64.1 64.1c12.5 12.5 12.5 32.8 0 45.3s-32.8 12.5-45.3 0l-90.2-90.2 44.5-9.9z";
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
    exports.faCrutches = exports.definition;
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

// src/_entries/faCrutches.js
var import_faCrutches = __toESM(require_faCrutches());
var export_faCrutches = import_faCrutches.definition;
export {
  export_faCrutches as faCrutches
};
