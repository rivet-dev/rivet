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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faLinkHorizontalSlash.js
var require_faLinkHorizontalSlash = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faLinkHorizontalSlash.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "link-horizontal-slash";
    var width = 640;
    var height = 512;
    var aliases = ["chain-horizontal-slash"];
    var unicode = "e1cc";
    var svgPathData = "M38.8 5.1C28.4-3.1 13.3-1.2 5.1 9.2S-1.2 34.7 9.2 42.9l592 464c10.4 8.2 25.5 6.3 33.7-4.1s6.3-25.5-4.1-33.7l-56.7-44.4c39.6-25.8 65.9-70.5 65.9-121.3c0-70.7-51.1-131-120.8-142.6l-1.9-.3c-17.4-2.9-33.9 8.9-36.8 26.3s8.9 33.9 26.3 36.8l1.9 .3c38.9 6.5 67.4 40.1 67.4 79.5c0 36.6-24.4 67.6-57.9 77.4L415.3 300.2c20.5-25.1 32.7-57.1 32.7-91.6C448 128.7 383.3 64 303.4 64L144.6 64c-9.4 0-18.5 .9-27.4 2.6L38.8 5.1zM195.6 128l107.8 0c44.5 0 80.6 36.1 80.6 80.6c0 19.7-7.1 38-19.1 52.1L195.6 128zM446.8 448l-81.2-64-28.9 0c-44.5 0-80.6-36.1-80.6-80.6c0-1.9 .1-3.7 .2-5.5l-55.5-43.8c-5.6 15.4-8.6 32.1-8.6 49.3C192 383.3 256.7 448 336.6 448l110.2 0zM30.4 119.9C11.3 144.4 0 175.2 0 208.6c0 70.7 51.1 131 120.8 142.6l1.9 .3c17.4 2.9 33.9-8.9 36.8-26.3s-8.9-33.9-26.3-36.8l-1.9-.3C92.5 281.6 64 248 64 208.6c0-18.5 6.2-35.5 16.6-49.1L30.4 119.9z";
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
    exports.faLinkHorizontalSlash = exports.definition;
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

// src/_entries/faChainHorizontalSlash.js
var import_faLinkHorizontalSlash = __toESM(require_faLinkHorizontalSlash());
var export_faChainHorizontalSlash = import_faLinkHorizontalSlash.definition;
export {
  export_faChainHorizontalSlash as faChainHorizontalSlash
};
