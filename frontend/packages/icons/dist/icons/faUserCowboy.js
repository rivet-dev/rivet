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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faUserCowboy.js
var require_faUserCowboy = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faUserCowboy.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "user-cowboy";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f8ea";
    var svgPathData = "M224 16c6.7 0 10.8-2.8 15.5-6.1C246.1 5.4 254 0 272 0c35.2 0 58.3 58.1 72 110.4C316.4 120 277 128 224 128s-92.4-8-120-17.6C117.7 58.1 140.8 0 176 0c18 0 25.9 5.4 32.5 9.9c4.7 3.3 8.8 6.1 15.5 6.1zM8.6 97.8c7-3.6 15.5-1.7 20.2 4.6c5.6 7.5 22.7 22 55 34.7C115.8 149.6 161.8 160 224 160s108.2-10.4 140.2-22.9c32.4-12.7 49.4-27.2 55-34.7c4.7-6.3 13.3-8.2 20.2-4.6s10.2 11.8 7.8 19.2c-9.3 27.9-35 62.4-72.5 89.9c-7.1 5.2-14.7 10.1-22.7 14.8c0 .8 0 1.5 0 2.3c0 70.7-57.3 128-128 128s-128-57.3-128-128c0-.8 0-1.5 0-2.3c-8-4.6-15.6-9.6-22.7-14.8C35.8 179.5 10.1 145 .8 117.1c-2.5-7.4 .8-15.6 7.8-19.2zM100.7 384l246.5 0c48.4 0 89.3 36.1 95.3 84.1l1 7.9c2.4 19.1-12.5 36-31.8 36L36.3 512C17 512 2.1 495.1 4.5 476l1-7.9c6-48 46.8-84.1 95.3-84.1z";
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
    exports.faUserCowboy = exports.definition;
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

// src/_entries/faUserCowboy.js
var import_faUserCowboy = __toESM(require_faUserCowboy());
var export_faUserCowboy = import_faUserCowboy.definition;
export {
  export_faUserCowboy as faUserCowboy
};
