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

// src/node_modules/@fortawesome/free-brands-svg-icons/faGitee.js
var require_faGitee = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faGitee.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "gitee";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e863";
    var svgPathData = "M256 512a256 256 0 1 1 0-512 256 256 0 1 1 0 512zM385.6 113.8l0 0-177 0c-52.4 0-94.8 42.4-94.8 94.8l0 177c0 7 5.7 12.6 12.6 12.6l186.5 0c47.1 0 85.3-38.2 85.3-85.3l0-72.7c0-7-5.7-12.6-12.6-12.6l-145.4 0c-7 0-12.6 5.7-12.6 12.6l0 31.6c0 6.6 5.1 12.1 11.6 12.6l1 0 88.5 0c6.6 0 12.1 5.1 12.6 11.6l0 1 0 6.3c0 20.9-17 37.9-37.9 37.9l-120.1 0c-7 0-12.6-5.7-12.6-12.6l0-120.1c0-20.4 16.1-37.1 36.4-37.9l1.6 0 177 0c7 0 12.6-5.7 12.6-12.6l0-31.6c0-7-5.6-12.6-12.6-12.6z";
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
    exports.faGitee = exports.definition;
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

// src/_entries/faGitee.js
var import_faGitee = __toESM(require_faGitee());
var export_faGitee = import_faGitee.definition;
export {
  export_faGitee as faGitee
};
