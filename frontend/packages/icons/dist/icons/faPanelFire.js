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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faPanelFire.js
var require_faPanelFire = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faPanelFire.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "panel-fire";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e42f";
    var svgPathData = "M0 96C0 60.7 28.7 32 64 32l320 0c35.3 0 64 28.7 64 64l0 32.2c-12.6-1.2-25.5 2.7-35.5 11.7c-9.9 8.9-19.4 18.1-28.5 27.5l0-39.4c0-17.7-14.3-32-32-32L96 96c-17.7 0-32 14.3-32 32l0 64c0 17.7 14.3 32 32 32l241.5 0c-4.4 6.2-8.5 12.5-12.3 18.7c-10.9 17.6-20.5 36.4-27.2 55.4c-3-1.4-6.4-2.1-9.9-2.1c-13.3 0-24 10.7-24 24s10.7 24 24 24l.2 0c-.1 2-.2 4.1-.2 6.1c0 49.3 18.6 95.2 49.6 129.9L64 480c-35.3 0-64-28.7-64-64L0 96zM120 320a24 24 0 1 0 -48 0 24 24 0 1 0 48 0zm72 24a24 24 0 1 0 0-48 24 24 0 1 0 0 48zM505.7 208.1L518 194.3c5.4-6.1 13.3-8.8 20.9-8.9c7.2 0 14.3 2.6 19.9 7.8c19.7 18.3 39.8 43.2 55 70.6C629 291.1 640 322 640 352c0 88.7-71.3 159.8-160 159.8c-89.6 0-160-71.3-160-159.8c0-37.3 16-73.4 36.8-104.5c20.9-31.3 47.5-59 70.9-80.2c5.7-5.2 13.1-7.7 20.3-7.5c14.1 .3 23.8 11.4 32.7 21.6c0 0 0 0 0 0c2 2.3 4 4.6 6 6.7l19 19.9zM544 400c0-36.5-37-73-54.8-88.4c-5.4-4.7-13.1-4.7-18.5 0C453 327 416 363.5 416 400c0 35.3 28.7 64 64 64s64-28.7 64-64z";
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
    exports.faPanelFire = exports.definition;
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

// src/_entries/faPanelFire.js
var import_faPanelFire = __toESM(require_faPanelFire());
var export_faPanelFire = import_faPanelFire.definition;
export {
  export_faPanelFire as faPanelFire
};
