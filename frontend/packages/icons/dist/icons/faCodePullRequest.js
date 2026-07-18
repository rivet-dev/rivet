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

// src/node_modules/@fortawesome/free-solid-svg-icons/faCodePullRequest.js
var require_faCodePullRequest = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faCodePullRequest.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "code-pull-request";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e13c";
    var svgPathData = "M328 24c0-9.7-5.8-18.5-14.8-22.2S293.9 .2 287 7L231 63c-9.4 9.4-9.4 24.6 0 33.9l56 56c6.9 6.9 17.2 8.9 26.2 5.2S328 145.7 328 136l0-24 24 0c17.7 0 32 14.3 32 32l0 214.7c-28.3 12.3-48 40.5-48 73.3 0 44.2 35.8 80 80 80s80-35.8 80-80c0-32.8-19.7-61-48-73.3L448 144c0-53-43-96-96-96l-24 0 0-24zM72 80a24 24 0 1 1 48 0 24 24 0 1 1 -48 0zm56 73.3c28.3-12.3 48-40.5 48-73.3 0-44.2-35.8-80-80-80S16 35.8 16 80c0 32.8 19.7 61 48 73.3l0 205.3c-28.3 12.3-48 40.5-48 73.3 0 44.2 35.8 80 80 80s80-35.8 80-80c0-32.8-19.7-61-48-73.3l0-205.3zM72 432a24 24 0 1 1 48 0 24 24 0 1 1 -48 0zm344-24a24 24 0 1 1 0 48 24 24 0 1 1 0-48z";
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
    exports.faCodePullRequest = exports.definition;
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

// src/_entries/faCodePullRequest.js
var import_faCodePullRequest = __toESM(require_faCodePullRequest());
var export_faCodePullRequest = import_faCodePullRequest.definition;
export {
  export_faCodePullRequest as faCodePullRequest
};
