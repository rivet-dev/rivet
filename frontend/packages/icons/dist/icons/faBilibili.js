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

// src/node_modules/@fortawesome/free-brands-svg-icons/faBilibili.js
var require_faBilibili = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faBilibili.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "bilibili";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e3d9";
    var svgPathData = "M488.6 104.1c16.7 18.1 24.4 39.7 23.3 65.7l0 202.4c-.4 26.4-9.2 48.1-26.5 65.1-17.2 17-39.1 25.9-65.5 26.7L92 464c-26.4-.8-48.2-9.8-65.3-27.2-17.1-17.4-26-40.3-26.7-68.6L0 169.8c.8-26 9.7-47.6 26.7-65.7 17.1-16.3 38.8-25.3 65.3-26.1l29.4 0-25.4-25.8c-5.7-5.7-8.6-13-8.6-21.8s2.9-16.1 8.6-21.8 13-8.6 21.9-8.6 16.1 2.9 21.9 8.6l73.3 69.4 88 0 74.5-69.4C381.7 2.9 389.2 0 398 0s16.1 2.9 21.9 8.6c5.7 5.7 8.6 13 8.6 21.8s-2.9 16.1-8.6 21.8L394.6 78 423.9 78c26.4 .8 48 9.8 64.7 26.1zm-38.8 69.7c-.4-9.6-3.7-17.4-10.7-23.5-5.2-6.1-14-9.4-22.7-9.8l-320.4 0c-9.6 .4-17.4 3.7-23.6 9.8-6.1 6.1-9.4 13.9-9.8 23.5l0 194.4c0 9.2 3.3 17 9.8 23.5s14.4 9.8 23.6 9.8l320.4 0c9.2 0 17-3.3 23.3-9.8s9.7-14.3 10.1-23.5l0-194.4zM185.5 216.5c6.3 6.3 9.7 14.1 10.1 23.2l0 33.3c-.4 9.2-3.7 16.9-9.8 23.2-6.2 6.3-14 9.5-23.6 9.5s-17.5-3.2-23.6-9.5-9.4-14-9.8-23.2l0-33.3c.4-9.1 3.8-16.9 10.1-23.2s13.2-9.6 23.3-10c9.2 .4 17 3.7 23.3 10zm191.5 0c6.3 6.3 9.7 14.1 10.1 23.2l0 33.3c-.4 9.2-3.7 16.9-9.8 23.2s-14 9.5-23.6 9.5-17.4-3.2-23.6-9.5c-7-6.3-9.4-14-9.7-23.2l0-33.3c.3-9.1 3.7-16.9 10-23.2s14.1-9.6 23.3-10c9.2 .4 17 3.7 23.3 10z";
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
    exports.faBilibili = exports.definition;
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

// src/_entries/faBilibili.js
var import_faBilibili = __toESM(require_faBilibili());
var export_faBilibili = import_faBilibili.definition;
export {
  export_faBilibili as faBilibili
};
