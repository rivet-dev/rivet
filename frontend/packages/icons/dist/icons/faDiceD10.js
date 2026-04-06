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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faDiceD10.js
var require_faDiceD10 = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faDiceD10.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "dice-d10";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f6cd";
    var svgPathData = "M209.7 16.3C212.4 8.3 206.4 0 198 0c-3.8 0-7.3 1.7-9.7 4.6L2.4 237C.8 238.9 0 241.3 0 243.8c0 7 6.4 12.1 13.2 10.6l117.6-26.1c5.5-1.2 9.9-5.2 11.7-10.6L209.7 16.3zM8.7 288.2C3.6 289.4 0 293.9 0 299.1c0 3.1 1.3 6.1 3.6 8.2L220.7 508.9c2.1 2 4.9 3.1 7.8 3.1c6.3 0 11.5-5.1 11.5-11.5l0-171.7c0-5.3-2.6-10.2-6.9-13.2l-78.7-54.5c-3.7-2.5-8.2-3.4-12.6-2.5L8.7 288.2zM272 500.5c0 6.3 5.1 11.5 11.5 11.5c2.9 0 5.7-1.1 7.8-3.1L508.4 307.3c2.3-2.1 3.6-5.1 3.6-8.2c0-5.2-3.6-9.8-8.7-10.9L370.2 258.7c-4.4-1-8.9-.1-12.6 2.5l-78.7 54.5c-4.3 3-6.9 7.9-6.9 13.2l0 171.7zM498.8 254.4c6.8 1.5 13.2-3.7 13.2-10.6c0-2.5-.8-4.9-2.4-6.8L323.7 4.6C321.4 1.7 317.8 0 314 0c-8.5 0-14.4 8.3-11.7 16.3l67.2 201.5c1.8 5.3 6.2 9.3 11.7 10.6l117.6 26.1zM246.6 6.8L174.8 222.3c-2.2 6.7 .2 14.2 6.1 18.2l66.1 45.7c5.5 3.8 12.7 3.8 18.2 0l66.1-45.7c5.8-4 8.3-11.5 6.1-18.2L265.4 6.8C264 2.7 260.3 0 256 0s-8 2.7-9.4 6.8z";
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
    exports.faDiceD10 = exports.definition;
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

// src/_entries/faDiceD10.js
var import_faDiceD10 = __toESM(require_faDiceD10());
var export_faDiceD10 = import_faDiceD10.definition;
export {
  export_faDiceD10 as faDiceD10
};
