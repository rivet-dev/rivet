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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faHelmetBattle.js
var require_faHelmetBattle = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faHelmetBattle.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "helmet-battle";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "f6eb";
    var svgPathData = "M80 17.1c.6-8.3-5.3-15.7-13.6-17s-16.1 4.1-18 12.2L.8 218.8c-.5 2.2-.8 4.5-.8 6.8C0 242.4 13.6 256 30.4 256l2.2 0c17.1 0 31.3-13.2 32.5-30.2L80 17.1zM320 352l0 136c0 18.4 20.8 30.1 36.5 20.5l144-88c9.2-5.6 13.5-16.7 10.6-27.1C494.1 333.6 480 270.2 480 208c0-85-89.1-149.2-153.1-189c-1.9-1.2-4-2.6-6.3-4.1C311.2 8.4 298.7 0 288 0s-23.2 8.4-32.7 14.9c-2.3 1.5-4.4 3-6.3 4.1C185.1 58.8 96 123 96 208c0 62.2-14.1 125.6-31.1 185.4c-3 10.4 1.4 21.4 10.6 27.1l144 88c15.7 9.6 36.5-2.1 36.5-20.5l0-192s0 0 0 0l0-28.5c0-6.9-4.4-13-10.9-15.2l-72.3-24.1c-7.6-2.5-12.7-9.6-12.7-17.6c0-10.3 8.3-18.6 18.6-18.6l218.8 0c10.3 0 18.6 8.3 18.6 18.6c0 8-5.1 15.1-12.7 17.6l-72.3 24.1c-6.5 2.2-10.9 8.3-10.9 15.2l0 84.5s0 0 0 0zM509.6 .2c-8.3 1.2-14.2 8.6-13.6 17l14.9 208.6c1.2 17 15.4 30.2 32.5 30.2l2.2 0c16.8 0 30.4-13.6 30.4-30.4c0-2.3-.3-4.6-.8-6.8L527.6 12.4c-1.9-8.1-9.7-13.5-18-12.2z";
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
    exports.faHelmetBattle = exports.definition;
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

// src/_entries/faHelmetBattle.js
var import_faHelmetBattle = __toESM(require_faHelmetBattle());
var export_faHelmetBattle = import_faHelmetBattle.definition;
export {
  export_faHelmetBattle as faHelmetBattle
};
