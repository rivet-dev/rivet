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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faPhoneIntercom.js
var require_faPhoneIntercom = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faPhoneIntercom.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "phone-intercom";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e434";
    var svgPathData = "M48 32l32 0c26.5 0 48 21.5 48 48l0 352c0 26.5-21.5 48-48 48l-32 0c-26.5 0-48-21.5-48-48L0 80C0 53.5 21.5 32 48 32zM160 96c0-35.3 28.7-64 64-64l224 0c35.3 0 64 28.7 64 64l0 320c0 35.3-28.7 64-64 64l-224 0c-35.3 0-64-28.7-64-64l0-320zM320 368a48 48 0 1 0 -96 0 48 48 0 1 0 96 0zm48-48c-8.8 0-16 7.2-16 16s7.2 16 16 16l64 0c8.8 0 16-7.2 16-16s-7.2-16-16-16l-64 0zm0 64c-8.8 0-16 7.2-16 16s7.2 16 16 16l64 0c8.8 0 16-7.2 16-16s-7.2-16-16-16l-64 0zM224 128l0 96c0 17.7 14.3 32 32 32l160 0c17.7 0 32-14.3 32-32l0-96c0-17.7-14.3-32-32-32L256 96c-17.7 0-32 14.3-32 32z";
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
    exports.faPhoneIntercom = exports.definition;
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

// src/_entries/faPhoneIntercom.js
var import_faPhoneIntercom = __toESM(require_faPhoneIntercom());
var export_faPhoneIntercom = import_faPhoneIntercom.definition;
export {
  export_faPhoneIntercom as faPhoneIntercom
};
