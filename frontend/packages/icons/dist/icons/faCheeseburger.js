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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBurgerCheese.js
var require_faBurgerCheese = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBurgerCheese.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "burger-cheese";
    var width = 512;
    var height = 512;
    var aliases = [127828, "cheeseburger"];
    var unicode = "f7f1";
    var svgPathData = "M32 194.9C32 211 45 224 61.1 224l389.8 0c16.1 0 29.1-13 29.1-29.1c0-1.9-.2-3.7-.6-5.6C474.1 168.3 433.2 32 256 32S37.9 168.3 32.6 189.3c-.5 1.9-.6 3.7-.6 5.6zM128 112a16 16 0 1 1 0 32 16 16 0 1 1 0-32zm240 16a16 16 0 1 1 32 0 16 16 0 1 1 -32 0zM256 80a16 16 0 1 1 0 32 16 16 0 1 1 0-32zM32 400l0 16c0 35.3 28.7 64 64 64l320 0c35.3 0 64-28.7 64-64l0-16c0-8.8-7.2-16-16-16L48 384c-8.8 0-16 7.2-16 16zM234 262.7c-6.5-4.4-14.2-6.7-22.1-6.7l-3.9 0-.1 0L64 256c-26.5 0-48 21.5-48 48s21.5 48 48 48l384 0c26.5 0 48-21.5 48-48s-21.5-48-48-48l-15.9 0-.1 0-3.9 0c-7.9 0-15.6 2.3-22.1 6.7l-68.2 45.5c-10.7 7.2-24.8 7.2-35.5 0L234 262.7z";
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
    exports.faBurgerCheese = exports.definition;
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

// src/_entries/faCheeseburger.js
var import_faBurgerCheese = __toESM(require_faBurgerCheese());
var export_faCheeseburger = import_faBurgerCheese.definition;
export {
  export_faCheeseburger as faCheeseburger
};
