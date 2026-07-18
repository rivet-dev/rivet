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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faMailboxFlagUp.js
var require_faMailboxFlagUp = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faMailboxFlagUp.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "mailbox-flag-up";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e5bb";
    var svgPathData = "M400 16c-29.8 0-54.9 20.4-62 48l-92.8 0C290.4 95.9 320 148.5 320 208l0 176c0 24.6-9.2 47-24.4 64L512 448c35.3 0 64-28.7 64-64l0-176c0-42.6-18.5-81-48-107.3L528 80c0-35.3-28.7-64-64-64l-64 0zm0 208c-8.8 0-16-7.2-16-16l0-96 0-32c0-8.8 7.2-16 16-16l64 0c8.8 0 16 7.2 16 16l0 32c0 8.8-7.2 16-16 16l-48 0 0 80c0 8.8-7.2 16-16 16zM0 208L0 384c0 35.3 28.7 64 64 64l160 0c35.3 0 64-28.7 64-64l0-176c0-79.5-64.5-144-144-144S0 128.5 0 208zm64 0c0-8.8 7.2-16 16-16l128 0c8.8 0 16 7.2 16 16s-7.2 16-16 16L80 224c-8.8 0-16-7.2-16-16z";
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
    exports.faMailboxFlagUp = exports.definition;
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

// src/_entries/faMailboxFlagUp.js
var import_faMailboxFlagUp = __toESM(require_faMailboxFlagUp());
var export_faMailboxFlagUp = import_faMailboxFlagUp.definition;
export {
  export_faMailboxFlagUp as faMailboxFlagUp
};
