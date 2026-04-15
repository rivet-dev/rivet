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

// src/node_modules/@fortawesome/free-solid-svg-icons/faUsersRectangle.js
var require_faUsersRectangle = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faUsersRectangle.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "users-rectangle";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e594";
    var svgPathData = "M64 32C28.7 32 0 60.7 0 96L0 416c0 35.3 28.7 64 64 64l448 0c35.3 0 64-28.7 64-64l0-320c0-35.3-28.7-64-64-64L64 32zm224 72a56 56 0 1 1 0 112 56 56 0 1 1 0-112zm0 152c53 0 96 43 96 96l0 24c0 13.3-10.7 24-24 24l-144 0c-13.3 0-24-10.7-24-24l0-24c0-53 43-96 96-96zm96-64a48 48 0 1 1 96 0 48 48 0 1 1 -96 0zM168 272.3c-15.2 22.8-24 50.2-24 79.7l0 24c0 8.4 1.4 16.5 4.1 24l-46.8 0C89.6 400 80 390.4 80 378.7L80 368c0-50.3 38.7-91.6 88-95.7zM427.9 400c2.7-7.5 4.1-15.6 4.1-24l0-24c0-29.5-8.8-56.9-24-79.7 49.3 4.1 88 45.3 88 95.7l0 10.7c0 11.8-9.6 21.3-21.3 21.3l-46.8 0zM96 192a48 48 0 1 1 96 0 48 48 0 1 1 -96 0z";
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
    exports.faUsersRectangle = exports.definition;
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

// src/_entries/faUsersRectangle.js
var import_faUsersRectangle = __toESM(require_faUsersRectangle());
var export_faUsersRectangle = import_faUsersRectangle.definition;
export {
  export_faUsersRectangle as faUsersRectangle
};
