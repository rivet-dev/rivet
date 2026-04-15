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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSignPosts.js
var require_faSignPosts = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSignPosts.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "sign-posts";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e625";
    var svgPathData = "M96 0C78.3 0 64 14.3 64 32L48 32C21.5 32 0 53.5 0 80L0 368c0 26.5 21.5 48 48 48l480 0c26.5 0 48-21.5 48-48l0-288c0-26.5-21.5-48-48-48l-16 0c0-17.7-14.3-32-32-32s-32 14.3-32 32L128 32c0-17.7-14.3-32-32-32zm32 448l-64 0 0 32c0 17.7 14.3 32 32 32s32-14.3 32-32l0-32zm320 0l0 32c0 17.7 14.3 32 32 32s32-14.3 32-32l0-32-64 0z";
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
    exports.faSignPosts = exports.definition;
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

// src/_entries/faSignPosts.js
var import_faSignPosts = __toESM(require_faSignPosts());
var export_faSignPosts = import_faSignPosts.definition;
export {
  export_faSignPosts as faSignPosts
};
