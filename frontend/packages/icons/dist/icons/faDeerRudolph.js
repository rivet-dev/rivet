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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faDeerRudolph.js
var require_faDeerRudolph = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faDeerRudolph.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "deer-rudolph";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "f78f";
    var svgPathData = "M256 16c0-8.8-7.2-16-16-16s-16 7.2-16 16l0 24c0 30.9 25.1 56 56 56l8 0 8 0 24 0-50.4 12.6c-8 2-13.6 9.2-13.6 17.4c0 9.9 8 18 18 18l38 0-9.6 32-106.1 0c-.8 0-1.7 0-2.5 0c-.6 0-1.2 0-1.7 0l-64 0-14.1 0-7.9 0C51.7 176 6.6 217.8 2.4 272L.1 302.2c-1 13.2 8.9 24.8 22.1 25.8c13 1 24.4-8.6 25.7-21.5c2.4 3.3 5.2 6.5 8.1 9.5c9 9 9 23.5 2.4 34.4c-17.4 28.6-21.5 63.9-10 96.3l15.5 44C68.4 503.4 80.5 512 94.1 512l33.9 0c22.1 0 37.5-21.8 30.2-42.6l-19.3-54.7c-2.2-6.2-.6-13 4-17.7l10.6-10.6c12.1-12.1 19.8-27.9 21.9-44.8c21 16.5 47.5 26.4 76.3 26.4l36.3 0 0 112c0 17.7 14.3 32 32 32l32 0c17.7 0 32-14.3 32-32l0-112 0-64 26.7-80 5.3 0 50.7 0c25 0 45.3-20.3 45.3-45.3c0-1.2 0-2.4-.1-3.6c2.6 .5 5.4 .8 8.1 .8c22.1 0 40-17.9 40-40s-17.9-40-40-40c-19.6 0-35.9 14.1-39.3 32.7L443.4 91.4c-.6-.6-1.1-1.1-1.7-1.6C459.7 80.4 472 61.7 472 40l0-24c0-8.8-7.2-16-16-16s-16 7.2-16 16l0 24c0 13.3-10.7 24-24 24l-38.7 0-30.6 0c3.5-7.3 5.4-15.4 5.4-24l0-24c0-8.8-7.2-16-16-16s-16 7.2-16 16l0 24c0 13.3-10.7 24-24 24l-8 0-8 0c-13.3 0-24-10.7-24-24l0-24zM384 144a16 16 0 1 1 32 0 16 16 0 1 1 -32 0z";
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
    exports.faDeerRudolph = exports.definition;
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

// src/_entries/faDeerRudolph.js
var import_faDeerRudolph = __toESM(require_faDeerRudolph());
var export_faDeerRudolph = import_faDeerRudolph.definition;
export {
  export_faDeerRudolph as faDeerRudolph
};
