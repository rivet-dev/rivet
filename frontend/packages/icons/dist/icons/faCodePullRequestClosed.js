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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCodePullRequestClosed.js
var require_faCodePullRequestClosed = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCodePullRequestClosed.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "code-pull-request-closed";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e3f9";
    var svgPathData = "M329.4 9.4c12.5-12.5 32.8-12.5 45.3 0L400 34.7 425.4 9.4c12.5-12.5 32.8-12.5 45.3 0s12.5 32.8 0 45.3L445.3 80l25.4 25.4c12.5 12.5 12.5 32.8 0 45.3s-32.8 12.5-45.3 0L400 125.3l-25.4 25.4c-12.5 12.5-32.8 12.5-45.3 0s-12.5-32.8 0-45.3L354.7 80 329.4 54.6c-12.5-12.5-12.5-32.8 0-45.3zM136 80A24 24 0 1 0 88 80a24 24 0 1 0 48 0zm56 0c0 32.8-19.7 61-48 73.3l0 205.3c28.3 12.3 48 40.5 48 73.3c0 44.2-35.8 80-80 80s-80-35.8-80-80c0-32.8 19.7-61 48-73.3l0-205.3C51.7 141 32 112.8 32 80C32 35.8 67.8 0 112 0s80 35.8 80 80zM112 456a24 24 0 1 0 0-48 24 24 0 1 0 0 48zm288 0a24 24 0 1 0 0-48 24 24 0 1 0 0 48zm0 56c-44.2 0-80-35.8-80-80c0-32.8 19.7-61 48-73.3L368 224c0-17.7 14.3-32 32-32s32 14.3 32 32l0 134.7c28.3 12.3 48 40.5 48 73.3c0 44.2-35.8 80-80 80z";
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
    exports.faCodePullRequestClosed = exports.definition;
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

// src/_entries/faCodePullRequestClosed.js
var import_faCodePullRequestClosed = __toESM(require_faCodePullRequestClosed());
var export_faCodePullRequestClosed = import_faCodePullRequestClosed.definition;
export {
  export_faCodePullRequestClosed as faCodePullRequestClosed
};
