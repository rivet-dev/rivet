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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faUserBountyHunter.js
var require_faUserBountyHunter = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faUserBountyHunter.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "user-bounty-hunter";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e2bf";
    var svgPathData = "M323 120C309.4 78 270 48 224 48s-85.4 30-99 72L323 120zm5.5 36.4L248 171.5l0 98.4 9.5-1.9c7.6-49.7 42.9-81.5 71.9-97.8l-.9-13.8zm-209 0l-.9 13.8c29 16.3 64.2 48.1 71.9 97.8l9.5 1.9 0-98.4-80.5-15.1zM96 70C123.1 27.7 170.5 0 224 0c80.1 0 146.5 62.2 151.7 142.2L383 256.5c1.3 20-12.4 37.9-32.1 41.8L250.1 318.4c-5.2 1-10.4 1.6-15.7 1.6l-20.9 0c-5.3 0-10.5-.5-15.7-1.6L97 298.3c-16.2-3.2-28.4-16-31.4-31.6c-1-3.4-1.6-7-1.6-10.7L64 16C64 7.2 71.2 0 80 0L96 0c8.8 0 16 7.2 16 16s-7.2 16-16 16l0 38zm38.4 282l179.2 0C387.8 352 448 412.2 448 486.4c0 14.1-11.5 25.6-25.6 25.6L25.6 512C11.5 512 0 500.5 0 486.4C0 412.2 60.2 352 134.4 352zm37.1 103.4c-3.7 2.8-1.7 8.6 2.9 8.6l99.2 0c4.6 0 6.6-5.9 2.9-8.6l-47.7-35.8c-2.8-2.1-6.8-2.1-9.6 0l-47.7 35.8z";
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
    exports.faUserBountyHunter = exports.definition;
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

// src/_entries/faUserBountyHunter.js
var import_faUserBountyHunter = __toESM(require_faUserBountyHunter());
var export_faUserBountyHunter = import_faUserBountyHunter.definition;
export {
  export_faUserBountyHunter as faUserBountyHunter
};
