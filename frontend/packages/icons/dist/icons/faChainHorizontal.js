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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faLinkHorizontal.js
var require_faLinkHorizontal = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faLinkHorizontal.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "link-horizontal";
    var width = 640;
    var height = 512;
    var aliases = ["chain-horizontal"];
    var unicode = "e1cb";
    var svgPathData = "M144.6 64C64.7 64 0 128.7 0 208.6c0 70.7 51.1 131 120.8 142.6l1.9 .3c17.4 2.9 33.9-8.9 36.8-26.3s-8.9-33.9-26.3-36.8l-1.9-.3C92.5 281.6 64 248 64 208.6c0-44.5 36.1-80.6 80.6-80.6l158.8 0c44.5 0 80.6 36.1 80.6 80.6c0 39.4-28.5 73-67.4 79.5l-1.9 .3c-17.4 2.9-29.2 19.4-26.3 36.8s19.4 29.2 36.8 26.3l1.9-.3c69.7-11.6 120.8-72 120.8-142.6C448 128.7 383.3 64 303.4 64L144.6 64zM495.4 448C575.3 448 640 383.3 640 303.4c0-70.7-51.1-131-120.8-142.6l-1.9-.3c-17.4-2.9-33.9 8.9-36.8 26.3s8.9 33.9 26.3 36.8l1.9 .3c38.9 6.5 67.4 40.1 67.4 79.5c0 44.5-36.1 80.6-80.6 80.6l-158.8 0c-44.5 0-80.6-36.1-80.6-80.6c0-39.4 28.5-73 67.4-79.5l1.9-.3c17.4-2.9 29.2-19.4 26.3-36.8s-19.4-29.2-36.8-26.3l-1.9 .3C243.1 172.4 192 232.7 192 303.4C192 383.3 256.7 448 336.6 448l158.8 0z";
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
    exports.faLinkHorizontal = exports.definition;
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

// src/_entries/faChainHorizontal.js
var import_faLinkHorizontal = __toESM(require_faLinkHorizontal());
var export_faChainHorizontal = import_faLinkHorizontal.definition;
export {
  export_faChainHorizontal as faChainHorizontal
};
