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

// src/node_modules/@fortawesome/free-solid-svg-icons/faJarWheat.js
var require_faJarWheat = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faJarWheat.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "jar-wheat";
    var width = 320;
    var height = 512;
    var aliases = [];
    var unicode = "e517";
    var svgPathData = "M32-8c0-13.3 10.7-24 24-24l208 0c13.3 0 24 10.7 24 24s-10.7 24-24 24L56 16C42.7 16 32 5.3 32-8zM0 128C0 92.7 28.7 64 64 64l192 0c35.3 0 64 28.7 64 64l0 320c0 35.3-28.7 64-64 64L64 512c-35.3 0-64-28.7-64-64L0 128zm112 32l-42.2 0c-3.2 0-5.8 2.6-5.8 5.8 0 32.1 26 58.2 58.2 58.2l75.6 0c32.1 0 58.2-26 58.2-58.2 0-3.2-2.6-5.8-5.8-5.8L208 160c-19.1 0-36.3 8.4-48 21.7-11.7-13.3-28.9-21.7-48-21.7zm48 117.7c-11.7-13.3-28.9-21.7-48-21.7l-42.2 0c-3.2 0-5.8 2.6-5.8 5.8 0 32.1 26 58.2 58.2 58.2l75.6 0c32.1 0 58.2-26 58.2-58.2 0-3.2-2.6-5.8-5.8-5.8L208 256c-19.1 0-36.3 8.4-48 21.7zM112 352l-42.2 0c-3.2 0-5.8 2.6-5.8 5.8 0 32.1 26 58.2 58.2 58.2l21.8 0 0 32c0 8.8 7.2 16 16 16s16-7.2 16-16l0-32 21.8 0c32.1 0 58.2-26 58.2-58.2 0-3.2-2.6-5.8-5.8-5.8L208 352c-19.1 0-36.3 8.4-48 21.7-11.7-13.3-28.9-21.7-48-21.7z";
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
    exports.faJarWheat = exports.definition;
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

// src/_entries/faJarWheat.js
var import_faJarWheat = __toESM(require_faJarWheat());
var export_faJarWheat = import_faJarWheat.definition;
export {
  export_faJarWheat as faJarWheat
};
