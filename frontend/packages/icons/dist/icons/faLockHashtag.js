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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faLockHashtag.js
var require_faLockHashtag = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faLockHashtag.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "lock-hashtag";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e423";
    var svgPathData = "M144 144l0 48 160 0 0-48c0-44.2-35.8-80-80-80s-80 35.8-80 80zM80 192l0-48C80 64.5 144.5 0 224 0s144 64.5 144 144l0 48 16 0c35.3 0 64 28.7 64 64l0 192c0 35.3-28.7 64-64 64L64 512c-35.3 0-64-28.7-64-64L0 256c0-35.3 28.7-64 64-64l16 0zm96 48c-8.8 0-16 7.2-16 16l0 32-32 0c-8.8 0-16 7.2-16 16s7.2 16 16 16l32 0 0 64-32 0c-8.8 0-16 7.2-16 16s7.2 16 16 16l32 0 0 32c0 8.8 7.2 16 16 16s16-7.2 16-16l0-32 64 0 0 32c0 8.8 7.2 16 16 16s16-7.2 16-16l0-32 32 0c8.8 0 16-7.2 16-16s-7.2-16-16-16l-32 0 0-64 32 0c8.8 0 16-7.2 16-16s-7.2-16-16-16l-32 0 0-32c0-8.8-7.2-16-16-16s-16 7.2-16 16l0 32-64 0 0-32c0-8.8-7.2-16-16-16zm16 144l0-64 64 0 0 64-64 0z";
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
    exports.faLockHashtag = exports.definition;
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

// src/_entries/faLockHashtag.js
var import_faLockHashtag = __toESM(require_faLockHashtag());
var export_faLockHashtag = import_faLockHashtag.definition;
export {
  export_faLockHashtag as faLockHashtag
};
